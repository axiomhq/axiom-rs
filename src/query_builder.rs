//! A simple (experimental) APL query builder.
//!
//! # Examples
//! ```
//! let query = QueryBuilder::new("my-dataset")
//!    .r#where("foo == 'bar'")
//!    .extend("baz = 1")
//!    .project(vec!["foo", "baz"])
//!    .take(10)
//!    .to_string();
//! assert_eq!(query, r#"['my-dataset']
//! | where foo == 'bar'
//! | extend baz = 1
//! | project foo, baz
//! | take 10"#);
//! ```
use std::{fmt, marker::PhantomData};

use crate::{
    datasets::{QueryOptions, QueryResult},
    Client, Error,
};

#[derive(Debug)]
enum Statement {
    Where(String),
    WhereAnd(String),
    WhereOr(String),
    Extend(Vec<String>),
    Project(Vec<String>),
    Take(i64),
    Summarize(String),
    By(Vec<String>),
    Count,
}

impl std::fmt::Display for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Statement::Where(expr) => {
                write!(f, "\n| where {}", expr)
            }
            Statement::WhereAnd(expr) => {
                write!(f, " and {}", expr)
            }
            Statement::WhereOr(expr) => {
                write!(f, " or {}", expr)
            }
            Statement::Extend(exprs) => {
                write!(f, "\n| extend {}", exprs.join(", "))
            }
            Statement::Project(exprs) => {
                write!(f, "\n| project {}", exprs.join(", "))
            }
            Statement::Take(count) => write!(f, "\n| take {}", count),
            Statement::Summarize(expr) => {
                write!(f, "\n| summarize {}", expr)
            }
            Statement::By(exprs) => {
                write!(f, " by {}", exprs.join(", "))
            }
            Statement::Count => write!(f, "\n| count"),
        }
    }
}

/// The APL query builder. For query methods, see [`StatefulQueryBuilder`].
#[derive(Debug)]
pub struct QueryBuilder {}

impl QueryBuilder {
    /// Create a new query builder for the given dataset.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(dataset_name: impl Into<String>) -> StatefulQueryBuilder<StateInitial> {
        StatefulQueryBuilder {
            dataset_name: dataset_name.into(),
            statements: Vec::new(),
            phantom: PhantomData,
        }
    }
}

/// This is the heart of the APL query builder. It keeps track of what operation
/// you ran last and allows you to extend it (i.e. chain `where` statements
/// using `and`/`or`).
#[derive(Debug)]
pub struct StatefulQueryBuilder<State> {
    dataset_name: String,
    statements: Vec<Statement>,
    phantom: PhantomData<State>,
}

/// A marker struct to indicate that the QueryBuilder is in its initial state.
#[derive(Debug)]
pub struct StateInitial;

impl<State> StatefulQueryBuilder<State> {
    /// Add a `where` statement to the query.
    ///
    /// See also [`StatefulQueryBuilder::and`] and [`StatefulQueryBuilder::or`].
    ///
    /// # Examples
    /// ```
    /// let query = QueryBuilder::new("my-dataset")
    ///     .r#where("foo == 'bar'")
    ///     .to_string();
    /// assert_eq!(query, r#"['my-dataset'] | where foo == 'bar'"#);
    /// ```
    pub fn r#where(mut self, expr: impl Into<String>) -> StatefulQueryBuilder<StateWhere> {
        self.statements.push(Statement::Where(expr.into()));
        StatefulQueryBuilder::<StateWhere> {
            dataset_name: self.dataset_name,
            statements: self.statements,
            phantom: PhantomData,
        }
    }

    /// Add an `extend` statement to the query.
    ///
    /// # Examples
    /// ```
    /// let query = QueryBuilder::new("my-dataset")
    ///     .extend("foo = 'bar'")
    ///     .to_string();
    /// assert_eq!(query, r#"['my-dataset'] | extend foo = 'bar'"#);
    /// ```
    pub fn extend(mut self, expr: impl StringOrVec) -> StatefulQueryBuilder<StateInitial> {
        self.statements.push(Statement::Extend(expr.into_vec()));
        StatefulQueryBuilder::<StateInitial> {
            dataset_name: self.dataset_name,
            statements: self.statements,
            phantom: PhantomData,
        }
    }

    /// Add a `project` statement to the query.
    ///
    /// # Examples
    /// ```
    /// let query = QueryBuilder::new("my-dataset")
    ///     .project("foo = 'bar'")
    ///     .to_string();
    /// assert_eq!(query, r#"['my-dataset'] | project foo = 'bar'"#);
    /// ```
    pub fn project(mut self, expr: impl StringOrVec) -> StatefulQueryBuilder<StateInitial> {
        self.statements.push(Statement::Project(expr.into_vec()));
        StatefulQueryBuilder::<StateInitial> {
            dataset_name: self.dataset_name,
            statements: self.statements,
            phantom: PhantomData,
        }
    }

    /// Add a `take` statement to the query.
    ///
    /// # Examples
    /// ```
    /// let query = QueryBuilder::new("my-dataset").take(10).to_string();
    /// assert_eq!(query, r#"['my-dataset'] | take 10"#);
    /// ```
    pub fn take(mut self, count: impl Into<i64>) -> StatefulQueryBuilder<StateInitial> {
        self.statements.push(Statement::Take(count.into()));
        StatefulQueryBuilder::<StateInitial> {
            dataset_name: self.dataset_name,
            statements: self.statements,
            phantom: PhantomData,
        }
    }

    /// Add a `summarize` statement to the query.
    ///
    /// See also [`StatefulQueryBuilder::by`].
    ///
    /// # Examples
    /// ```
    /// let query = QueryBuilder::new("my-dataset")
    ///     .summarize("count()")
    ///     .to_string();
    /// assert_eq!(query, r#"['my-dataset'] | summarize count()"#);
    /// ```
    pub fn summarize(mut self, expr: impl Into<String>) -> StatefulQueryBuilder<StateSummarize> {
        self.statements.push(Statement::Summarize(expr.into()));
        StatefulQueryBuilder::<StateSummarize> {
            dataset_name: self.dataset_name,
            statements: self.statements,
            phantom: PhantomData,
        }
    }

    /// Add a `count` statement to the query.
    ///
    /// # Examples
    /// ```
    /// let query = QueryBuilder::new("my-dataset").count().to_string();
    /// assert_eq!(query, r#"['my-dataset'] | count"#);
    /// ```
    pub fn count(mut self) -> StatefulQueryBuilder<StateInitial> {
        self.statements.push(Statement::Count);
        StatefulQueryBuilder::<StateInitial> {
            dataset_name: self.dataset_name,
            statements: self.statements,
            phantom: PhantomData,
        }
    }

    /// Run the query using the given client.
    pub async fn run(
        self,
        client: Client,
        opts: impl Into<Option<QueryOptions>>,
    ) -> Result<QueryResult, Error> {
        let query = self.to_string();
        client.query(&query, opts).await
    }
}

impl<State> fmt::Display for StatefulQueryBuilder<State> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "['{}']{}",
            self.dataset_name,
            self.statements
                .iter()
                .map(|stmt| stmt.to_string())
                .collect::<Vec<_>>()
                .join("")
        )
    }
}

/// A marker struct to indicate that the QueryBuilder's last statement is `where`.
#[derive(Debug)]
pub struct StateWhere;

/// The marker struct for [`StateWhere`].
pub trait Where {}

impl Where for StateWhere {}

impl<State> StatefulQueryBuilder<State>
where
    State: Where,
{
    /// Add an `and` statement to the current where statement.
    ///
    /// See also [`StatefulQueryBuilder::where`].
    ///
    /// # Examples
    /// ```
    /// let query = QueryBuilder::new("my-dataset")
    ///     .r#where("foo == 'bar'")
    ///     .and("baz == 'qux'")
    ///     .to_string();
    /// assert_eq!(query, r#"['my-dataset'] | where foo == 'bar' and baz == 'qux'"#);
    /// ```
    pub fn and(mut self, expr: impl Into<String>) -> Self {
        self.statements.push(Statement::WhereAnd(expr.into()));
        self
    }

    /// Add an `or` statement to the current where statement.
    ///
    /// See also [`StatefulQueryBuilder::where`].
    ///
    /// # Examples
    /// ```
    /// let query = QueryBuilder::new("my-dataset")
    ///     .r#where("foo == 'bar'")
    ///     .or("baz == 'qux'")
    ///     .to_string();
    /// assert_eq!(query, r#"['my-dataset'] | where foo == 'bar' or baz == 'qux'"#);
    pub fn or(mut self, expr: impl Into<String>) -> Self {
        self.statements.push(Statement::WhereOr(expr.into()));
        self
    }
}

/// A marker struct to indicate that the QueryBuilder's last statement is
/// `summarize`.
#[derive(Debug)]
pub struct StateSummarize;

/// The marker struct for [`StateSummarize`].
pub trait Summarize {}

impl Summarize for StateSummarize {}

impl<State> StatefulQueryBuilder<State>
where
    State: Summarize,
{
    /// Add a `by` statement to the current summarize statement.
    ///
    /// See also [`StatefulQueryBuilder::summarize`].
    ///
    /// # Examples
    /// ```
    /// let query = QueryBuilder::new("my-dataset")
    ///     .summarize("count()")
    ///     .by("foo")
    ///     .to_string();
    /// assert_eq!(query, r#"['my-dataset'] | summarize count() by foo"#);
    /// ```
    pub fn by(mut self, fields: impl StringOrVec) -> StatefulQueryBuilder<StateInitial> {
        self.statements.push(Statement::By(fields.into_vec()));
        StatefulQueryBuilder::<StateInitial> {
            dataset_name: self.dataset_name,
            statements: self.statements,
            phantom: PhantomData,
        }
    }
}

/// A trait to convert a string or a vector of strings into a vector of strings.
/// It's used in methods where we want to accept one or more strings.
pub trait StringOrVec {
    fn into_vec(self) -> Vec<String>;
}

impl StringOrVec for String {
    fn into_vec(self) -> Vec<String> {
        vec![self]
    }
}

impl StringOrVec for &str {
    fn into_vec(self) -> Vec<String> {
        vec![self.to_string()]
    }
}

impl StringOrVec for Vec<&str> {
    fn into_vec(self) -> Vec<String> {
        self.into_iter().map(|s| s.to_string()).collect()
    }
}

impl StringOrVec for Vec<String> {
    fn into_vec(self) -> Vec<String> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder() {
        let query = QueryBuilder::new("users")
            .r#where("name == 'John'")
            .and("age == 30")
            .or("age == 40")
            .extend(vec!["height = 84", "isYoung = age < 30"])
            .project("weight = 78")
            .take(10)
            .summarize("avg(price)")
            .by(vec!["bin_auto(_time)", "customer_name"])
            .count()
            .to_string();

        assert_eq!(
            query,
            r#"['users']
| where name == 'John' and age == 30 or age == 40
| extend height = 84, isYoung = age < 30
| project weight = 78
| take 10
| summarize avg(price) by bin_auto(_time), customer_name
| count"#
        );
    }
}
