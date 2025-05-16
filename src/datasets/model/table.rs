use std::fmt::{self, Display};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::value::Value as JsonValue;

/// Specifies the order a queries result will be in.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Order {
    /// Field to order on.
    pub field: String,
    /// If the field is ordered desending.
    pub desc: bool,
}

/// The datatype of the column in a table.
#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
#[serde(transparent)]
pub struct FieldType {
    /// The name of the field type.
    pub name: String,
}

impl Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl AsRef<str> for FieldType {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

/// An aggregation.
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Agg {
    /// Name of the aggregation
    pub name: String,
    /// Fields that the aggregation is applied to
    #[serde(default)]
    pub fields: Vec<String>,
    /// Arguments to the aggregation
    #[serde(default)]
    pub args: Vec<JsonValue>,
}

impl Display for Agg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.name, self.fields.join(", "))
    }
}

/// A field of an Axiom dataset.
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Field {
    /// Name is the unique name of the field.
    pub name: String,
    /// Type is the datatype of the field.
    #[serde(rename = "type")]
    pub typ: FieldType,
    /// Aggregation details if field is an aggregate
    pub agg: Option<Agg>,
}

impl Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.typ)
    }
}

/// The source dataset of a table.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    /// The name of the source.
    pub name: String,
}

impl Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// A grouping as part of a table.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    /// The name of the group.
    pub name: String,
}

impl Display for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// The range over which a given field is queried.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    /// The field that is ranged over.
    pub field: String,
    /// Start time.
    pub start: DateTime<Utc>,
    /// End time.
    pub end: DateTime<Utc>,
}

impl Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}..{}]", self.field, self.start, self.end)
    }
}

/// The bucketing applied to a table.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    /// The field that is bucketed.
    pub field: String,
    /// The size of the bucket.
    pub size: u64,
}

impl Display for Bucket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}]", self.field, self.size)
    }
}

/// A table in the query result.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    /// The name of the table.
    pub name: String,
    /// The source(s) of the table.
    pub sources: Vec<Source>,
    /// The fields of the table.
    pub fields: Vec<Field>,
    /// The order of the events.
    pub order: Vec<Order>,
    /// The groups, if any.
    pub groups: Vec<Group>,
    /// The range of this table.
    pub range: Option<Range>,
    /// The buckets, if any.
    pub buckets: Option<Bucket>,
    /// The columns of this table.
    pub columns: Vec<Vec<JsonValue>>,
}

impl Table {
    /// Returns true if the first column is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the maximum length of the first column
    pub fn len(&self) -> usize {
        self.columns.first().map(Vec::len).unwrap_or_default()
    }

    /// Returns a single row from the table.
    #[must_use]
    pub fn get_row(&self, row: usize) -> Option<Row> {
        if self.len() > row {
            Some(Row { table: self, row })
        } else {
            None
        }
    }

    /// Returns an iterator over the rows.
    #[must_use]
    pub fn iter(&self) -> RowIter {
        RowIter {
            table: self,
            row: 0,
        }
    }
}

impl<'table> IntoIterator for &'table Table {
    type Item = Row<'table>;
    type IntoIter = RowIter<'table>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over the rows of a table.
pub struct RowIter<'table> {
    table: &'table Table,
    row: usize,
}
impl<'table> Iterator for RowIter<'table> {
    type Item = Row<'table>;

    fn next(&mut self) -> Option<Self::Item> {
        let row = self.table.get_row(self.row)?;
        self.row += 1;
        Some(row)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.table.len();
        (size - self.row, Some(size - self.row))
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.table.len() - self.row
    }

    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        if self.table.is_empty() {
            None
        } else {
            self.table.get_row(self.table.len() - 1)
        }
    }
}

/// A row in a table.
pub struct Row<'table> {
    table: &'table Table,
    row: usize,
}

impl<'table> Row<'table> {
    /// Returns the value of the row by name
    #[must_use]
    pub fn get_field(&self, field: &str) -> Option<&JsonValue> {
        let mut index = None;

        for (i, f) in self.table.fields.iter().enumerate() {
            if f.name == field {
                index = Some(i);
                break;
            }
        }

        self.get(index?)
    }
    /// Returns the value of the row.
    #[must_use]
    pub fn get(&self, column: usize) -> Option<&JsonValue> {
        self.table.columns.get(column).and_then(|c| c.get(self.row))
    }
    /// Returns the value of the row as a string.
    #[must_use]
    pub fn fields(&self) -> &[Field] {
        &self.table.fields
    }
    /// Returns an iterator over the fields of the row.
    #[must_use]
    pub fn iter(&self) -> FieldIter<'table> {
        FieldIter {
            table: self.table,
            row: self.row,
            index: 0,
        }
    }
}

impl<'table> IntoIterator for &Row<'table> {
    type Item = Option<&'table JsonValue>;
    type IntoIter = FieldIter<'table>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over the fields of a row.
pub struct FieldIter<'table> {
    table: &'table Table,
    row: usize,
    index: usize,
}

impl<'table> Iterator for FieldIter<'table> {
    type Item = Option<&'table JsonValue>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.table.columns.len() {
            return None;
        }
        let value = self
            .table
            .columns
            .get(self.index)
            .and_then(|c| c.get(self.row));
        self.index += 1;
        Some(value)
    }
}
