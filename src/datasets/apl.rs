pub struct Empty;
pub struct Populated {
    dataset_name: String,
    tabular_operators: Vec<TabularOperator>,
}
pub struct WhereClause {
    dataset_name: String,
    tabular_operators: Vec<TabularOperator>,
}

pub enum TabularOperator {
    Where {
        left: String,
        op: String,
        right: String,
    },
    And {
        left: String,
        op: String,
        right: String,
    },
    Or {
        left: String,
        op: String,
        right: String,
    },
    Count,
    Project {
        fields: Vec<String>,
    },
    Summarize {
        aggregation: String,
        by: String,
    },
}

#[derive(Debug)]
pub struct AplBuilder<S> {
    state: S,
}

pub fn builder() -> AplBuilder<Empty> {
    AplBuilder { state: Empty }
}

impl AplBuilder<Empty> {
    pub fn dataset<S>(self, dataset_name: S) -> AplBuilder<Populated>
    where
        S: Into<String>,
    {
        AplBuilder {
            state: Populated {
                dataset_name: dataset_name.into(),
                tabular_operators: vec![],
            },
        }
    }
}

impl WithTabularOperators for AplBuilder<Populated> {
    fn into_parts(self) -> (String, Vec<TabularOperator>) {
        (self.state.dataset_name, self.state.tabular_operators)
    }

    fn push_tabular_operator(&mut self, action: TabularOperator) {
        self.state.tabular_operators.push(action);
    }
}

impl WithTabularOperators for AplBuilder<WhereClause> {
    fn into_parts(self) -> (String, Vec<TabularOperator>) {
        (self.state.dataset_name, self.state.tabular_operators)
    }

    fn push_tabular_operator(&mut self, action: TabularOperator) {
        self.state.tabular_operators.push(action);
    }
}

#[doc(hidden)]
pub trait WithTabularOperators {
    fn into_parts(self) -> (String, Vec<TabularOperator>);
    fn push_tabular_operator(&mut self, action: TabularOperator);
}

impl TabularOperators for AplBuilder<Populated> {}
impl TabularOperators for AplBuilder<WhereClause> {}

macro_rules! where_fn(
    ($name:ident, $op:expr) => (
        fn $name<L, R>(self, left: L, right: R) -> AplBuilder<WhereClause>
        where
            L: Into<String>,
            R: Into<String>,
        {
        let (dataset_name, mut tabular_operators) = self.into_parts();
        tabular_operators.push(TabularOperator::Where {
            left: left.into(),
            op: $op.into(),
            right: right.into(),
        });
        AplBuilder {
            state: WhereClause {
                dataset_name,
                tabular_operators,
            },
        }
        }
    )
);

macro_rules! and_fn(
    ($name:ident, $op:expr) => (
        pub fn $name<L, R>(self, left: L, right: R) -> AplBuilder<WhereClause>
        where
            L: Into<String>,
            R: Into<String>,
        {
        let (dataset_name, mut tabular_operators) = self.into_parts();
        tabular_operators.push(TabularOperator::And {
            left: left.into(),
            op: $op.into(),
            right: right.into(),
        });
        AplBuilder {
            state: WhereClause {
                dataset_name,
                tabular_operators,
            },
        }
        }
    )
);

macro_rules! or_fn(
    ($name:ident, $op:expr) => (
        pub fn $name<L, R>(self, left: L, right: R) -> AplBuilder<WhereClause>
        where
            L: Into<String>,
            R: Into<String>,
        {
        let (dataset_name, mut tabular_operators) = self.into_parts();
        tabular_operators.push(TabularOperator::Or {
            left: left.into(),
            op: $op.into(),
            right: right.into(),
        });
        AplBuilder {
            state: WhereClause {
                dataset_name,
                tabular_operators,
            },
        }
        }
    )
);

pub trait TabularOperators: WithTabularOperators + Sized {
    fn where_raw<L, O, R>(self, left: L, op: O, right: R) -> AplBuilder<WhereClause>
    where
        L: Into<String>,
        O: Into<String>,
        R: Into<String>,
    {
        let (dataset_name, mut tabular_operators) = self.into_parts();
        tabular_operators.push(TabularOperator::Where {
            left: left.into(),
            op: op.into(),
            right: right.into(),
        });
        AplBuilder {
            state: WhereClause {
                dataset_name,
                tabular_operators,
            },
        }
    }

    where_fn!(where_eq, "==");
    where_fn!(where_ne, "!=");
    where_fn!(where_gt, ">");
    where_fn!(where_ge, ">=");
    where_fn!(where_lt, "<");
    where_fn!(where_le, "<=");

    fn count(self) -> AplBuilder<Populated> {
        let (dataset_name, mut tabular_operators) = self.into_parts();
        tabular_operators.push(TabularOperator::Count);
        AplBuilder {
            state: Populated {
                dataset_name,
                tabular_operators,
            },
        }
    }

    fn project<S>(mut self, fields: Vec<S>) -> Self
    where
        S: Into<String>,
    {
        let fields = fields.into_iter().map(|f| f.into()).collect();
        self.push_tabular_operator(TabularOperator::Project { fields });
        self
    }

    fn summarize<A, B>(mut self, aggregation: A, by: B) -> Self
    where
        A: Into<String>,
        B: Into<String>,
    {
        self.push_tabular_operator(TabularOperator::Summarize {
            aggregation: aggregation.into(),
            by: by.into(),
        });
        self
    }

    fn build(self) -> String {
        let (dataset_name, actions) = self.into_parts();

        let mut apl = format!("['{}']", dataset_name);

        actions.iter().for_each(|action| match action {
            TabularOperator::Where { left, op, right } => {
                apl.push_str(&format!(r#" | where {} {} "{}""#, left, op, right));
            }
            TabularOperator::And { left, op, right } => {
                apl.push_str(&format!(r#" and {} {} "{}""#, left, op, right));
            }
            TabularOperator::Or { left, op, right } => {
                apl.push_str(&format!(r#" or {} {} "{}""#, left, op, right));
            }
            TabularOperator::Count => {
                apl.push_str(" | count");
            }
            TabularOperator::Project { fields } => {
                apl.push_str(&format!(" | project {}", fields.join(", ")));
            }
            TabularOperator::Summarize { aggregation, by } => {
                apl.push_str(&format!(" | summarize {} by {}", aggregation, by));
            }
        });

        apl
    }
}

impl AplBuilder<WhereClause> {
    pub fn and_raw<L, O, R>(self, left: L, op: O, right: R) -> AplBuilder<WhereClause>
    where
        L: Into<String>,
        O: Into<String>,
        R: Into<String>,
    {
        let (dataset_name, mut tabular_operators) = self.into_parts();
        tabular_operators.push(TabularOperator::And {
            left: left.into(),
            op: op.into(),
            right: right.into(),
        });
        AplBuilder {
            state: WhereClause {
                dataset_name,
                tabular_operators,
            },
        }
    }

    and_fn!(and_eq, "==");
    and_fn!(and_ne, "!=");
    and_fn!(and_gt, ">");
    and_fn!(and_ge, ">=");
    and_fn!(and_lt, "<");
    and_fn!(and_le, "<=");

    pub fn or_raw<L, O, R>(self, left: L, op: O, right: R) -> AplBuilder<WhereClause>
    where
        L: Into<String>,
        O: Into<String>,
        R: Into<String>,
    {
        let (dataset_name, mut tabular_operators) = self.into_parts();
        tabular_operators.push(TabularOperator::Or {
            left: left.into(),
            op: op.into(),
            right: right.into(),
        });
        AplBuilder {
            state: WhereClause {
                dataset_name,
                tabular_operators,
            },
        }
    }

    or_fn!(or_eq, "==");
    or_fn!(or_ne, "!=");
    or_fn!(or_gt, ">");
    or_fn!(or_ge, ">=");
    or_fn!(or_lt, "<");
    or_fn!(or_le, "<=");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_simple() {
        let apl = builder().dataset("foo").build();
        assert_eq!("['foo']", apl);
    }

    #[test]
    fn test_builder_advanced() {
        let apl = builder()
            .dataset("foo")
            .where_eq("foo", "bar")
            .and_eq("bar", "baz")
            .or_eq("baz", "qux")
            .count()
            .project(vec!["foo"])
            .summarize("count()", "bin_auto(_time)")
            .build();
        assert_eq!(
            r#"['foo'] | where foo == "bar" | count | project foo | summarize count() by bin_auto(_time)"#,
            apl
        );
    }
}
