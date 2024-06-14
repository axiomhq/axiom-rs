#![allow(deprecated)] // we need this to be allowed to declare depricated code
use bitflags::bitflags;
use bitflags_serde_shim::impl_serde_for_bitflags;
use chrono::{DateTime, Duration, Utc};
use http::header::HeaderValue;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use serde_json::value::Value as JsonValue;
use std::{
    collections::HashMap,
    fmt::{self, Display},
    ops::Add,
    str::FromStr,
};

use crate::serde::{deserialize_null_default, empty_string_as_none};

/// The default field the server looks for a time to use as
/// ingestion time. If not present, the server will set the ingestion time by
/// itself.
pub static TIMESTAMP_FIELD: &str = "_time";

/// All supported content types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContentType {
    /// JSON treats the data as JSON array.
    Json,
    /// NDJSON treats the data as newline delimited JSON objects. Preferred
    /// format.
    NdJson,
    /// CSV treats the data as CSV content.
    Csv,
}

impl ContentType {
    /// Returns the content type as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::Json => "application/json",
            ContentType::NdJson => "application/x-ndjson",
            ContentType::Csv => "text/csv",
        }
    }
}

impl Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ContentType {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "application/json" => Ok(ContentType::Json),
            "application/x-ndjson" => Ok(ContentType::NdJson),
            "text/csv" => Ok(ContentType::Csv),
            _ => Err(crate::error::Error::InvalidContentType(s.to_string())),
        }
    }
}

impl From<ContentType> for HeaderValue {
    fn from(content_type: ContentType) -> Self {
        HeaderValue::from_static(content_type.as_str())
    }
}

/// All supported content encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContentEncoding {
    /// Identity marks the data as not being encoded.
    Identity,
    /// GZIP marks the data as being gzip encoded.
    Gzip,
    /// Zstd marks the data as being zstd encoded.
    Zstd,
}

impl ContentEncoding {
    /// Returns the content encoding as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentEncoding::Identity => "",
            ContentEncoding::Gzip => "gzip",
            ContentEncoding::Zstd => "zstd",
        }
    }
}

impl Display for ContentEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ContentEncoding {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "" => Ok(ContentEncoding::Identity),
            "gzip" => Ok(ContentEncoding::Gzip),
            "zstd" => Ok(ContentEncoding::Zstd),
            _ => Err(crate::error::Error::InvalidContentEncoding(s.to_string())),
        }
    }
}

impl From<ContentEncoding> for HeaderValue {
    fn from(content_encoding: ContentEncoding) -> Self {
        HeaderValue::from_static(content_encoding.as_str())
    }
}

/// An Axiom dataset.
#[derive(Serialize, Deserialize, Debug)]
pub struct Dataset {
    /// The name of the dataset.
    pub name: String,
    /// The description of the dataset.
    pub description: String,
    /// The ID of the user who created the dataset.
    #[serde(rename = "who")]
    pub created_by: String,
    /// The time the dataset was created at.
    #[serde(rename = "created")]
    pub created_at: DateTime<Utc>,
    // ignored: integrationConfigs, integrationFilters, quickQueries
}

/// A field of an Axiom dataset.
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Field {
    /// Name is the unique name of the field.
    pub name: String,
    /// Description is the description of the field.
    pub description: String,
    /// Type is the datatype of the field.
    #[serde(rename = "type")]
    pub typ: String,
    /// Unit is the unit of the field.
    pub unit: String,
    /// Hidden describes if the field is hidden or not.
    pub hidden: bool,
}

/// Details of the information stored in a dataset.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Stat {
    /// The unique name of the dataset.
    pub name: String,
    /// The number of blocks of the dataset.
    #[deprecated(
        since = "0.8.0",
        note = "This field will be removed in a future version."
    )]
    pub num_blocks: u64,
    /// The number of events of the dataset.
    pub num_events: u64,
    /// The number of fields of the dataset.
    pub num_fields: u32,
    /// The amount of data stored in the dataset.
    pub input_bytes: u64,
    /// The amount of data stored in the dataset formatted in a human
    /// readable format.
    #[deprecated(
        since = "0.8.0",
        note = "This field will be removed in a future version."
    )]
    pub input_bytes_human: String,
    /// The amount of compressed data stored in the dataset.
    pub compressed_bytes: u64,
    /// The amount of compressed data stored in the
    /// dataset formatted in a human readable format.
    #[deprecated(
        since = "0.8.0",
        note = "This field will be removed in a future version."
    )]
    pub compressed_bytes_human: String,
    /// The time of the oldest event stored in the dataset.
    pub min_time: Option<DateTime<Utc>>,
    /// The time of the newest event stored in the dataset.
    pub max_time: Option<DateTime<Utc>>,
    /// The ID of the user who created the dataset.
    #[serde(rename = "who")]
    #[deprecated(
        since = "0.8.0",
        note = "This field will be removed in a future version."
    )]
    pub created_by: Option<String>,
    /// The time the dataset was created at.
    #[serde(rename = "created")]
    pub created_at: DateTime<Utc>,
}

/// Details of the information stored inside a dataset including the fields.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    /// The stats of the dataset.
    #[serde(flatten)]
    pub stat: Stat,
    /// The fields of the dataset.
    pub fields: Vec<Field>,
}

#[derive(Serialize, Debug)]
pub(crate) struct TrimRequest {
    #[serde(rename = "maxDuration")]
    max_duration: String,
}

impl TrimRequest {
    pub(crate) fn new(duration: Duration) -> Self {
        TrimRequest {
            max_duration: format!("{}s", duration.num_seconds()),
        }
    }
}

/// The result of a trim operation.
#[deprecated(
    since = "0.8.0",
    note = "The trim response will be removed in a future version."
)]
#[derive(Deserialize, Debug)]
pub struct TrimResult {
    /// The amount of blocks deleted by the trim operation.
    #[deprecated(
        since = "0.4.0",
        note = "This field is deprecated and will be removed in a future version."
    )]
    #[serde(rename = "numDeleted")]
    #[allow(deprecated, warnings)]
    pub blocks_deleted: u64,
}

/// Returned on event ingestion operation.
#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct IngestStatus {
    /// Amount of events that have been ingested.
    pub ingested: u64,
    /// Amount of events that failed to ingest.
    pub failed: u64,
    /// Ingestion failures, if any.
    pub failures: Vec<IngestFailure>,
    /// Number of bytes processed.
    pub processed_bytes: u64,
    /// Amount of blocks created.
    #[deprecated(
        since = "0.8.0",
        note = "This field will be removed in a future version."
    )]
    pub blocks_created: u32,
    /// The length of the Write-Ahead Log.
    #[deprecated(
        since = "0.8.0",
        note = "This field will be removed in a future version."
    )]
    pub wal_length: u32,
}

impl Add for IngestStatus {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut failures = self.failures;
        failures.extend(other.failures);

        #[allow(deprecated)]
        Self {
            ingested: self.ingested + other.ingested,
            failed: self.failed + other.failed,
            failures,
            processed_bytes: self.processed_bytes + other.processed_bytes,
            blocks_created: self.blocks_created + other.blocks_created,
            wal_length: other.wal_length,
        }
    }
}

/// Ingestion failure of a single event.
#[derive(Serialize, Deserialize, Debug)]
pub struct IngestFailure {
    /// Timestamp of the event that failed to ingest.
    pub timestamp: DateTime<Utc>,
    /// Error that made the event fail to ingest.
    pub error: String,
}

/// Used to create a dataset.
#[derive(Serialize, Debug)]
pub(crate) struct DatasetCreateRequest {
    /// Restricted to 128 bytes of [a-zA-Z0-9] and special characters "-", "_"
    /// and ".". Special characters cannot be a prefix or suffix. The prefix
    /// cannot be "axiom-".
    pub name: String,
    /// Description of the dataset.
    pub description: String,
}

/// Used to update a dataset.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct DatasetUpdateRequest {
    /// Description of the dataset to update.
    pub description: String,
}

/// A query that gets executed on a dataset.
/// If you're looking for the analytics, check out [`Query`].
#[derive(Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    /// The APL of the query to execute
    pub apl: String,
    /// Start time of the query.
    pub start_time: Option<DateTime<Utc>>,
    /// End time of the query.
    pub end_time: Option<DateTime<Utc>>,
    /// cursor for the query
    pub cursor: Option<String>,
    /// Specifies whether the event that matches the cursor should be included or not
    pub include_cursor: bool,
}

// QueryParams is the part of `QueryOptions` that is added to the request url.
#[derive(Serialize, Debug, Default)]
pub(crate) struct QueryParams {
    #[serde(rename = "nocache")]
    pub no_cache: bool,
    #[serde(rename = "saveAsKind")]
    pub save: bool,
    pub format: AplResultFormat,
}

/// The optional parameters to APL query methods.
#[derive(Debug)]
pub struct QueryOptions {
    /// The start time of the query.
    pub start_time: Option<DateTime<Utc>>,
    /// The end time of the query.
    pub end_time: Option<DateTime<Utc>>,
    /// The cursor for use in pagination.
    pub cursor: Option<String>,
    /// Specifies whether the event that matches the cursor should be
    /// included in the result.
    pub include_cursor: bool,

    /// Omits the query cache.
    pub no_cache: bool,
    /// Save the query on the server, if set to `true`. The ID of the saved query
    /// is returned with the query result as part of the response.
    // NOTE: The server automatically sets the query kind to "apl" for queries
    // going // to the "/_apl" query endpoint. This allows us to set any value
    // for the // `saveAsKind` query param. For user experience, we use a bool
    // here instead of forcing the user to set the value to `query.APL`.
    pub save: bool,
    /// Format specifies the format of the APL query. Defaults to Legacy.
    pub format: AplResultFormat,
}

impl Default for QueryOptions {
    fn default() -> Self {
        QueryOptions {
            start_time: None,
            end_time: None,
            cursor: None,
            include_cursor: false,
            no_cache: false,
            save: false,
            format: AplResultFormat::Legacy,
        }
    }
}

/// The result format of an APL query.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "lowercase")]
pub enum AplResultFormat {
    /// Legacy result format
    Legacy,
}

impl Default for AplResultFormat {
    fn default() -> Self {
        AplResultFormat::Legacy
    }
}

/// The kind of a query.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "lowercase")]
pub enum QueryKind {
    /// Analytics query
    Analytics,
    /// Streaming query
    Stream,
    /// APL query,   Read-only, don't use this for requests.
    Apl,
}

impl Default for QueryKind {
    fn default() -> Self {
        QueryKind::Analytics
    }
}

/// A query that gets executed on a dataset.
/// If you're looking for the APL query, check out [`Query`].
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LegacyQuery {
    /// Start time of the query.
    #[serde(deserialize_with = "empty_string_as_none")]
    pub start_time: Option<DateTime<Utc>>,
    /// End time of the query.
    #[serde(deserialize_with = "empty_string_as_none")]
    pub end_time: Option<DateTime<Utc>>,
    /// Resolution of the queries graph. Valid values are the queries time
    /// range / 100 at maximum and / 1000 at minimum. Use zero value for
    /// serve-side auto-detection.
    #[serde(default)]
    pub resolution: String, // TODO: Implement custom type to {de,}serialize to/from go string
    /// Aggregations performed as part of the query.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub aggregations: Vec<Aggregation>,
    /// Filter applied on the queried results.
    pub filter: Option<Filter>,
    /// Field names to group the query results by.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub group_by: Vec<String>,
    /// Order rules that specify the order of the query result.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub order: Vec<Order>,
    /// Number of results returned from the query.
    #[serde(default)]
    pub limit: u32,
    /// Virtual fields that can be referenced by aggregations, filters and
    /// orders.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub virtual_fields: Vec<VirtualField>,
    /// Pricections for the query result.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub projections: Vec<Projection>,
    /// The query cursor. Should be set to the cursor returned with a previous
    /// query result, if it was parital.
    #[serde(default)]
    pub cursor: String,
    /// Return the Cursor as part of the query result.
    #[serde(default)]
    pub include_cursor: bool,
    /// Used to get more results of a previous query. It is not valid for starred
    /// queries or otherwise stored queries.
    #[serde(default)]
    pub continuation_token: String,
}

/// A field that is projected to the query result.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Projection {
    /// The name of the field to project.
    pub field: String,
    /// The alias to reference the projected field by.
    pub alias: Option<String>,
}

/// Supported aggregation operations.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AggregationOp {
    /// [count](https://axiom.co/docs/apl/aggregation-function/statistical-functions#count())
    Count,
    /// [dcount](https://axiom.co/docs/apl/aggregation-function/statistical-functions#dcount())
    CountDistinct,
    /// [make_set](https://axiom.co/docs/apl/aggregation-function/statistical-functions#make-set())
    MakeSet,
    /// [make_set_if](https://axiom.co/docs/apl/aggregation-function/statistical-functions#make-set-if())
    MakeSetIf,

    /// [sum](https://axiom.co/docs/apl/aggregation-function/statistical-functions#sum())
    Sum,
    /// [avg](https://axiom.co/docs/apl/aggregation-function/statistical-functions#avg())
    Avg,
    /// [min](https://axiom.co/docs/apl/aggregation-function/statistical-functions#min())
    Min,
    /// [max](https://axiom.co/docs/apl/aggregation-function/statistical-functions#max())
    Max,
    /// [topk](https://axiom.co/docs/apl/aggregation-function/statistical-functions#topk())
    Topk,
    /// [percentile](https://axiom.co/docs/apl/aggregation-function/statistical-functions#percentile(),-percentiles-array())
    Percentiles,
    /// [histogram](https://axiom.co/docs/apl/aggregation-function/statistical-functions#histogram())
    Histogram,
    /// [stdev](https://axiom.co/docs/apl/aggregation-function/statistical-functions#stdev())
    StandardDeviation,
    /// [variance](https://axiom.co/docs/apl/aggregation-function/statistical-functions#variance())
    Variance,
    /// [argmin](https://axiom.co/docs/apl/aggregation-function/statistical-functions#argmin())
    ArgMin,
    /// [argmax](https://axiom.co/docs/apl/aggregation-function/statistical-functions#argmax())
    ArgMax,

    /// Read-only. Not to be used for query requests. Only in place to support the APL query result.
    /// [countif](https://axiom.co/docs/apl/aggregation-function/statistical-functions#countif())
    CountIf,
    /// Read-only. Not to be used for query requests. Only in place to support the APL query result.
    /// [dcountif](https://axiom.co/docs/apl/aggregation-function/statistical-functions#dcountif())
    DistinctIf,

    /// Unknown aggregation operation.
    Unknown(String),
}

impl Serialize for AggregationOp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            Self::Count => "count",
            Self::CountDistinct => "distinct",
            Self::MakeSet => "makeset",
            Self::MakeSetIf => "makesetif",
            Self::Sum => "sum",
            Self::Avg => "avg",
            Self::Min => "min",
            Self::Max => "max",
            Self::Topk => "topk",
            Self::Percentiles => "percentiles",
            Self::Histogram => "histogram",
            Self::StandardDeviation => "stdev",
            Self::Variance => "variance",
            Self::ArgMin => "argmin",
            Self::ArgMax => "argmax",
            Self::CountIf => "countif",
            Self::DistinctIf => "distinctif",
            Self::Unknown(ref s) => s,
        })
    }
}

struct AggregationOpVisitor;

impl<'de> Visitor<'de> for AggregationOpVisitor {
    type Value = AggregationOp;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a valid aggregation op string")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match s {
            "count" => Ok(Self::Value::Count),
            "distinct" => Ok(Self::Value::CountDistinct),
            "makeset" => Ok(Self::Value::MakeSet),
            "makesetif" => Ok(Self::Value::MakeSetIf),
            "sum" => Ok(Self::Value::Sum),
            "avg" => Ok(Self::Value::Avg),
            "min" => Ok(Self::Value::Min),
            "max" => Ok(Self::Value::Max),
            "topk" => Ok(Self::Value::Topk),
            "percentiles" => Ok(Self::Value::Percentiles),
            "histogram" => Ok(Self::Value::Histogram),
            "stdev" => Ok(Self::Value::StandardDeviation),
            "variance" => Ok(Self::Value::Variance),
            "argmin" => Ok(Self::Value::ArgMin),
            "argmax" => Ok(Self::Value::ArgMax),
            "countif" => Ok(Self::Value::CountIf),
            "distinctif" => Ok(Self::Value::DistinctIf),
            aggregation => Ok(Self::Value::Unknown(aggregation.to_string())),
        }
    }
}

impl<'de> Deserialize<'de> for AggregationOp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(AggregationOpVisitor {})
    }
}

/// Aggregations are applied to a query.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Aggregation {
    /// The alias for the aggregation.
    pub alias: Option<String>,
    /// The operation of the aggregation.
    pub op: AggregationOp,
    /// The field to aggregate on.
    pub field: String,
    /// Argument to the aggregation.
    /// Only valid for `OpCountDistinctIf`, `OpTopk`, `OpPercentiles` and
    /// `OpHistogram` aggregations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argument: Option<JsonValue>,
}

/// Supported filter operations. Supported types listed behind each operation.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "lowercase")]
pub enum FilterOp {
    /// Logical AND
    And,
    /// Logical OR
    Or,
    /// Logical NOT
    Not,

    // Works for strings and numbers.
    /// equality (string, number)
    #[serde(rename = "==")]
    Equal,
    /// negated equality (string, number)
    #[serde(rename = "!=")]
    NotEqual,
    /// existance (string, number)
    Exists,
    /// negated existance (string, number)
    NotExists,

    // Only works for numbers.
    /// greater than (number)
    #[serde(rename = ">")]
    GreaterThan,
    /// greater than or equal (number)
    #[serde(rename = ">=")]
    GreaterThanEqual,
    /// less than (number)
    #[serde(rename = "<")]
    LessThan,
    /// less than or equal (number)
    #[serde(rename = "<=")]
    LessThanEqual,

    // Only works for strings.
    /// starts with (string)
    StartsWith,
    /// negated starts with (string)
    NotStartsWith,
    /// ends with (string)
    EndsWith,
    /// negated ends with (string)
    NotEndsWith,
    /// regular expression (string)
    Regexp,
    /// negated regular expression (string)
    NotRegexp,

    // Works for strings and arrays.
    /// contains (string, array)
    Contains,
    /// negated contains (string, array)
    NotContains,
}

/// A filter is applied to a query.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    /// The operation of the filter.
    pub op: FilterOp,
    /// The field to filter on.
    pub field: String,
    /// The value to filter against.
    pub value: JsonValue,
    /// If the filter should be case insensitive.
    #[serde(default)]
    pub case_insensitive: bool,
    /// Child filters that are applied to the filter.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub children: Vec<Filter>,
}

impl Default for Filter {
    fn default() -> Self {
        Filter {
            op: FilterOp::Equal,
            field: "".to_string(),
            value: JsonValue::Null,
            case_insensitive: false,
            children: vec![],
        }
    }
}

/// Specifies the order a queries result will be in.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Order {
    /// Field to order on.
    pub field: String,
    /// If the field is ordered desending.
    pub desc: bool,
}

/// A VirtualField is not part of a dataset and its value is derived from an
/// expression. Aggregations, filters and orders can reference this field like
/// any other field.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct VirtualField {
    /// Alias the virtual field is referenced by.
    pub alias: String,
    /// Expression which specifies the virtual fields value.
    pub expr: String,
}

/// The parameters for a query.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct LegacyQueryOptions {
    /// Duration of the stream
    #[serde(rename = "streaming-duration")]
    pub streaming_duration: Option<String>, // TODO: Implement custom type to {de,}serialize to/from go string
    /// If the query should not be cached.
    #[serde(rename = "no-cache")]
    pub no_cache: bool,
    /// The kind to save the query wit.
    #[serde(rename = "saveAsKind")]
    pub save_as_kind: QueryKind,
}

/// The query result. It embeds the APL request in the result it created.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    /// The query request.
    pub request: LegacyQuery,
    // NOTE: The following is copied from QueryResult. Maybe we should have a macro?
    /// The status of the query result.
    pub status: QueryStatus,
    /// The datasets that were queried.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub dataset_names: Vec<String>,
    /// The events that matched the query.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub matches: Vec<Entry>,
    /// The time series buckets.
    pub buckets: Timeseries,
    /// The ID of the query that generated this result when it was saved on the
    /// server. This is only set when the query was send with the `SaveKind`
    /// option specified.
    #[serde(skip)]
    pub saved_query_id: Option<String>,
}

/// The legacy result of a query.
#[derive(Serialize, Deserialize, Debug)]
pub struct LegacyQueryResult {
    /// The status of the query result.
    pub status: QueryStatus,
    /// The events that matched the query.
    pub matches: Vec<Entry>,
    /// The time series buckets.
    pub buckets: Timeseries,
    /// The ID of the query that generated this result when it was saved on the
    /// server. This is only set when the query was send with the `SaveKind`
    /// option specified.
    #[serde(skip)]
    pub saved_query_id: Option<String>,
}

/// The status of a query result.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QueryStatus {
    /// The duration it took the query to execute.
    pub elapsed_time: u64,
    /// The amount of blocks that have been examined by the query.
    pub blocks_examined: u64,
    /// The amount of rows that have been examined by the query.
    pub rows_examined: u64,
    /// The amount of rows that matched the query.
    pub rows_matched: u64,
    /// The amount of groups returned by the query.
    pub num_groups: u32,
    /// True if the query result is a partial result.
    pub is_partial: bool,
    /// Populated when IsPartial is true, must be passed to the next query
    /// request to retrieve the next result set.
    pub continuation_token: Option<String>,
    /// True if the query result is estimated.
    #[serde(default)]
    pub is_estimate: bool,
    /// The status of the cache.
    pub cache_status: CacheStatus,
    /// The timestamp of the oldest block examined.
    pub min_block_time: DateTime<Utc>,
    /// The timestamp of the newest block examined.
    pub max_block_time: DateTime<Utc>,
    /// Messages associated with the query.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub messages: Vec<QueryMessage>,
    /// Row id of the newest row, as seen server side.
    pub max_cursor: Option<String>,
    /// Row id of the oldest row, as seen server side.
    pub min_cursor: Option<String>,
}

bitflags! {
    /// The cache status of the query.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct CacheStatus: u32 {
        /// cache miss
        const Miss = 1;
        /// Filtered rows
        const Materialized = 2;
        /// Aggregated and grouped records
        const Results = 4;
        /// WAL is cached
        const WalCached = 8;
    }
}
impl_serde_for_bitflags!(CacheStatus);

/// A message that is returned in the status of a query.
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryMessage {
    priority: QueryMessagePriority,
    count: u32,
    code: QueryMessageCode,
    text: Option<String>,
}

/// The priority of a query message.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Copy)]
#[non_exhaustive]
#[serde(rename_all = "lowercase")]
pub enum QueryMessagePriority {
    /// Trace message priority.
    Trace,
    /// Debug message priority.
    Debug,
    /// Info message priority.
    Info,
    /// Warn message priority.
    Warn,
    /// Error message priority.
    Error,
    /// Fatal message priority.
    Fatal,
}

/// The code of a message that is returned in the status of a query.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Copy)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum QueryMessageCode {
    /// Failed to finalize a virtual field.
    VirtualFieldFinalizeError,
    /// Missing column in the dataset.
    MissingColumn,
    /// Default limit warning.
    DefaultLimitWarning,
    /// License limit for query warning.
    LicenseLimitForQueryWarning,
    /// Other unknown error
    #[serde(other)]
    Unknown,
}

/// An event that matched a query and is thus part of the result set.
#[derive(Serialize, Deserialize, Debug)]
pub struct Entry {
    /// The time the event occurred. Matches SysTime if not specified during
    /// ingestion.
    #[serde(rename = "_time")]
    pub time: DateTime<Utc>,
    /// The time the event was recorded on the server.
    #[serde(rename = "_sysTime")]
    pub sys_time: DateTime<Utc>,
    /// The unique ID of the event row.
    #[serde(rename = "_rowId")]
    pub row_id: String,
    /// Contains the raw data of the event (with filters and aggregations
    /// applied).
    pub data: HashMap<String, JsonValue>,
}

/// A queried time series.
#[derive(Serialize, Deserialize, Debug)]
pub struct Timeseries {
    /// The intervals that build a time series.
    pub series: Vec<Interval>,
    /// The totals of the time series.
    pub totals: Vec<EntryGroup>,
}

/// The interval of queried time series.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Interval {
    /// The start time of the interval.
    pub start_time: DateTime<Utc>,
    /// The end time of the interval.
    pub end_time: DateTime<Utc>,
    /// The groups of the interval.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub groups: Vec<EntryGroup>,
}

/// A group of queried event.
#[derive(Serialize, Deserialize, Debug)]
pub struct EntryGroup {
    /// The unique ID of the group.
    pub id: u64,
    /// The data of the group.
    pub group: HashMap<String, JsonValue>,
    /// The aggregations of the group.
    pub aggregations: Vec<EntryGroupAgg>,
}

/// An aggregation which is part of a group of queried events.
#[derive(Serialize, Deserialize, Debug)]
pub struct EntryGroupAgg {
    /// The alias of the aggregation.
    #[serde(rename = "op")]
    pub alias: String,
    /// The value of the aggregation.
    pub value: JsonValue,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_aggregation_op() {
        let enum_repr = AggregationOp::Count;
        let json_repr = r#""count""#;
        assert_eq!(serde_json::to_string(&enum_repr).unwrap(), json_repr);
        assert_eq!(
            serde_json::from_str::<AggregationOp>(json_repr).unwrap(),
            enum_repr
        );
    }

    #[test]
    fn test_filter_op() {
        let enum_repr = FilterOp::And;
        let json_repr = r#""and""#;
        assert_eq!(serde_json::to_string(&enum_repr).unwrap(), json_repr);
        assert_eq!(
            serde_json::from_str::<FilterOp>(json_repr).unwrap(),
            enum_repr
        );

        let enum_repr = FilterOp::Equal;
        let json_repr = r#""==""#;
        assert_eq!(serde_json::to_string(&enum_repr).unwrap(), json_repr);
        assert_eq!(
            serde_json::from_str::<FilterOp>(json_repr).unwrap(),
            enum_repr
        );
    }
}
