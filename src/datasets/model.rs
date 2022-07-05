use chrono::{DateTime, Duration, Utc};
use http::header::HeaderValue;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{
    de::{self, Error, Unexpected, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use serde_json::value::Value as JsonValue;
use std::{
    collections::HashMap,
    convert::TryFrom,
    fmt::{self, Display},
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
#[derive(Serialize, Deserialize, Debug, PartialEq)]
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
    pub num_blocks: u64,
    /// The number of events of the dataset.
    pub num_events: u64,
    /// The number of fields of the dataset.
    pub num_fields: u32,
    /// The amount of data stored in the dataset.
    pub input_bytes: u64,
    /// The amount of data stored in the dataset formatted in a human
    /// readable format.
    pub input_bytes_human: String,
    /// The amount of compressed data stored in the dataset.
    pub compressed_bytes: u64,
    /// The amount of compressed data stored in the
    /// dataset formatted in a human readable format.
    pub compressed_bytes_human: String,
    /// The time of the oldest event stored in the dataset.
    pub min_time: Option<DateTime<Utc>>,
    /// The time of the newest event stored in the dataset.
    pub max_time: Option<DateTime<Utc>>,
    /// The ID of the user who created the dataset.
    #[serde(rename = "who")]
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
#[derive(Deserialize, Debug)]
pub struct TrimResult {
    /// The amount of blocks deleted by the trim operation.
    #[serde(rename = "numDeleted")]
    pub blocks_deleted: u64,
}

/// Returned on event ingestion operation.
#[derive(Deserialize, Debug)]
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
    pub blocks_created: u32,
    /// The length of the Write-Ahead Log.
    pub wal_length: u32,
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
pub struct DatasetCreateRequest {
    /// Restricted to 128 bytes of [a-zA-Z0-9] and special characters "-", "_"
    /// and ".". Special characters cannot be a prefix or suffix. The prefix
    /// cannot be "axiom-".
    pub name: String,
    /// Description of the dataset.
    pub description: String,
}

/// Used to update a dataset.
#[derive(Serialize, Deserialize, Debug)]
pub struct DatasetUpdateRequest {
    /// Description of the dataset to update.
    pub description: String,
}

/// A query that gets executed on a dataset.
/// If you're looking for the analytics, check out [`Query`].
#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AplQuery {
    pub apl: String,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

// AplQueryParams is the part of `AplOptions` that is added to the request url.
#[derive(Serialize, Debug, Default)]
pub(crate) struct AplQueryParams {
    #[serde(rename = "nocache")]
    pub no_cache: bool,
    #[serde(rename = "saveAsKind")]
    pub save: bool,
    pub format: AplResultFormat,
}

/// The optional parameters to APL query methods.
pub struct AplOptions {
    /// The start time of the query.
    pub start_time: Option<DateTime<Utc>>,
    // The end time of the query.
    pub end_time: Option<DateTime<Utc>>,

    // Omits the query cache.
    pub no_cache: bool,
    /// Save the query on the server, if set to `true`. The ID of the saved query
    /// is returned with the query result as part of the response.
    // NOTE: The server automatically sets the query kind to "apl" for queries
    // going // to the "/_apl" query endpoint. This allows us to set any value
    // for the // `saveAsKind` query param. For user experience, we use a bool
    // here instead of forcing the user to set the value to `query.APL`.
    pub save: bool,
    // Format specifies the format of the APL query. Defaults to Legacy.
    pub format: AplResultFormat,
}

impl Default for AplOptions {
    fn default() -> Self {
        AplOptions {
            start_time: None,
            end_time: None,
            no_cache: false,
            save: false,
            format: AplResultFormat::Legacy,
        }
    }
}

/// The result format of an APL query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AplResultFormat {
    Legacy,
}

impl Serialize for AplResultFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            AplResultFormat::Legacy => serializer.serialize_str("legacy"),
        }
    }
}

impl Default for AplResultFormat {
    fn default() -> Self {
        AplResultFormat::Legacy
    }
}

/// A query stored inside the query history.
#[derive(Deserialize, Debug)]
pub struct HistoryQuery {
    /// The unique id of the starred query.
    pub id: String,
    /// The kind of the starred query.
    pub kind: QueryKind,
    /// The dataset the starred query belongs to.
    pub dataset: String,
    /// Owner is the ID of the starred queries owner. Can be a user or team ID.
    pub owner: Option<String>,
    /// Query is the actual query.
    pub query: HistoryQueryRequest,
    /// The time the starred query was created at.
    pub created: DateTime<Utc>,
}

/// A [`HistoryQuery`] can embed either an [`AplQuery`] or a [`Query`].
#[derive(Deserialize, Debug)]
#[serde(untagged)]
#[non_exhaustive]
pub enum HistoryQueryRequest {
    Apl(AplQuery),
    Query(Box<Query>),
}

/// The kind of a query.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum QueryKind {
    Analytics,
    Stream,
    Apl, // Read-only, don't use this for requests.
}

impl Serialize for QueryKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            QueryKind::Analytics => serializer.serialize_str("analytics"),
            QueryKind::Stream => serializer.serialize_str("stream"),
            QueryKind::Apl => serializer.serialize_str("apl"),
        }
    }
}

impl<'de> Deserialize<'de> for QueryKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match String::deserialize(deserializer)?.as_str() {
            "analytics" => Ok(QueryKind::Analytics),
            "stream" => Ok(QueryKind::Stream),
            "apl" => Ok(QueryKind::Apl),
            _ => Err(D::Error::custom("unknown query kind")),
        }
    }
}

impl Default for QueryKind {
    fn default() -> Self {
        QueryKind::Analytics
    }
}

/// A query that gets executed on a dataset.
/// If you're looking for the APL query, check out [`AplQuery`].
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Query {
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

impl Default for Query {
    fn default() -> Self {
        Query {
            start_time: None,
            end_time: None,
            resolution: "".to_string(),
            aggregations: vec![],
            filter: None,
            group_by: vec![],
            order: vec![],
            limit: 0,
            virtual_fields: vec![],
            projections: vec![],
            cursor: "".to_string(),
            include_cursor: false,
            continuation_token: "".to_string(),
        }
    }
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
    Count,
    CountDistinct,

    // Only works for numbers.
    Sum,
    Avg,
    Min,
    Max,
    Topk,
    Percentiles,
    Histogram,
}

impl Serialize for AggregationOp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            AggregationOp::Count => "count",
            AggregationOp::CountDistinct => "distinct",
            AggregationOp::Sum => "sum",
            AggregationOp::Avg => "avg",
            AggregationOp::Min => "min",
            AggregationOp::Max => "max",
            AggregationOp::Topk => "topk",
            AggregationOp::Percentiles => "percentiles",
            AggregationOp::Histogram => "histogram",
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
            "count" => Ok(AggregationOp::Count),
            "distinct" => Ok(AggregationOp::CountDistinct),
            "sum" => Ok(AggregationOp::Sum),
            "avg" => Ok(AggregationOp::Avg),
            "min" => Ok(AggregationOp::Min),
            "max" => Ok(AggregationOp::Max),
            "topk" => Ok(AggregationOp::Topk),
            "percentiles" => Ok(AggregationOp::Percentiles),
            "histogram" => Ok(AggregationOp::Histogram),
            _ => Err(de::Error::invalid_value(Unexpected::Str(s), &self)),
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

/// Supported filter operations.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FilterOp {
    And,
    Or,
    Not,

    // Works for strings and numbers.
    Equal,
    NotEqual,
    Exists,
    NotExists,

    // Only works for numbers.
    GreaterThan,
    GreaterThanEqual,
    LessThan,
    LessThanEqual,

    // Only works for strings.
    StartsWith,
    NotStartsWith,
    EndsWith,
    NotEndsWith,
    Regexp,
    NotRegexp,

    // Works for strings and arrays.
    Contains,
    NotContains,
}

impl Serialize for FilterOp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            FilterOp::And => "and",
            FilterOp::Or => "or",
            FilterOp::Not => "not",
            FilterOp::Equal => "==",
            FilterOp::NotEqual => "!=",
            FilterOp::Exists => "exists",
            FilterOp::NotExists => "not-exists",
            FilterOp::GreaterThan => ">",
            FilterOp::GreaterThanEqual => ">=",
            FilterOp::LessThan => "<",
            FilterOp::LessThanEqual => "<=",
            FilterOp::StartsWith => "starts-with",
            FilterOp::NotStartsWith => "not-starts-with",
            FilterOp::EndsWith => "ends-with",
            FilterOp::NotEndsWith => "not-ends-with",
            FilterOp::Regexp => "regexp",
            FilterOp::NotRegexp => "not-regexp",
            FilterOp::Contains => "contains",
            FilterOp::NotContains => "not-contains",
        })
    }
}

struct FilterOpVisitor;

impl<'de> Visitor<'de> for FilterOpVisitor {
    type Value = FilterOp;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a valid filter op string")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match s {
            "and" => Ok(FilterOp::And),
            "or" => Ok(FilterOp::Or),
            "not" => Ok(FilterOp::Not),
            "==" => Ok(FilterOp::Equal),
            "!=" => Ok(FilterOp::NotEqual),
            "exists" => Ok(FilterOp::Exists),
            "not-exists" => Ok(FilterOp::NotExists),
            ">" => Ok(FilterOp::GreaterThan),
            ">=" => Ok(FilterOp::GreaterThanEqual),
            "<" => Ok(FilterOp::LessThan),
            "<=" => Ok(FilterOp::LessThanEqual),
            "starts-with" => Ok(FilterOp::StartsWith),
            "not-starts-with" => Ok(FilterOp::NotStartsWith),
            "ends-with" => Ok(FilterOp::EndsWith),
            "not-ends-with" => Ok(FilterOp::NotEndsWith),
            "regexp" => Ok(FilterOp::Regexp),
            "not-regexp" => Ok(FilterOp::NotRegexp),
            "contains" => Ok(FilterOp::Contains),
            "not-contains" => Ok(FilterOp::NotContains),
            _ => Err(de::Error::invalid_value(Unexpected::Str(s), &self)),
        }
    }
}

impl<'de> Deserialize<'de> for FilterOp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(FilterOpVisitor {})
    }
}

/// A filter is applied to a query.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    pub op: FilterOp,
    pub field: String,
    pub value: JsonValue,
    #[serde(default)]
    pub case_insensitive: bool,
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
pub struct QueryOptions {
    #[serde(rename = "streaming-duration")]
    pub streaming_duration: Option<String>, // TODO: Implement custom type to {de,}serialize to/from go string
    #[serde(rename = "no-cache")]
    pub no_cache: bool,
    #[serde(rename = "saveAsKind")]
    pub save_as_kind: QueryKind,
}

/// The result of an APL query. It embeds the APL request in the result it
/// created.
#[derive(Serialize, Deserialize, Debug)]
pub struct AplQueryResult {
    /// The query request.
    pub request: Query,
    // NOTE: The following is copied from QueryResult. Maybe we should have a macro?
    /// The status of the query result.
    pub status: QueryStatus,
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

/// The result of a query.
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryResult {
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
}

/// The cache status of the query.
#[derive(IntoPrimitive, TryFromPrimitive, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum CacheStatus {
    Miss = 1,
    Materialized = 2, // Filtered rows
    Results = 4,      // Aggregated and grouped records
}

impl Serialize for CacheStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8((*self).into())
    }
}

impl<'de> Deserialize<'de> for CacheStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: u8 = Deserialize::deserialize(deserializer)?;
        Self::try_from(value).map_err(serde::de::Error::custom)
    }
}

/// A message that is returned in the status of a query.
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryMessage {
    priority: QueryMessagePriority,
    count: u32,
    code: QueryMessageCode,
    text: String,
}

/// The priority of a query message.
#[derive(IntoPrimitive, TryFromPrimitive, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum QueryMessagePriority {
    Trace = 1,
    Debug = 2,
    Info = 3,
    Warn = 4,
    Error = 5,
    Fatal = 6,
}

impl Serialize for QueryMessagePriority {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8((*self).into())
    }
}

impl<'de> Deserialize<'de> for QueryMessagePriority {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: u8 = Deserialize::deserialize(deserializer)?;
        Self::try_from(value).map_err(serde::de::Error::custom)
    }
}

/// The code of a message that is returned in the status of a query.
#[derive(IntoPrimitive, TryFromPrimitive, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum QueryMessageCode {
    Unknown = 0,
    VirtualFieldFinalizeError = 1,
    LicenseLimitForQueryWarning = 2,
    DefaultLimitWarning = 3,
}

impl Serialize for QueryMessageCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8((*self).into())
    }
}

impl<'de> Deserialize<'de> for QueryMessageCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: u8 = Deserialize::deserialize(deserializer)?;
        Self::try_from(value).map_err(serde::de::Error::custom)
    }
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
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub groups: Vec<EntryGroup>,
}

/// A group of queried event.
#[derive(Serialize, Deserialize, Debug)]
pub struct EntryGroup {
    pub id: u64,
    pub group: HashMap<String, JsonValue>,
    pub aggregations: Vec<EntryGroupAgg>,
}

/// An aggregation which is part of a group of queried events.
#[derive(Serialize, Deserialize, Debug)]
pub struct EntryGroupAgg {
    #[serde(rename = "op")]
    pub alias: String,
    pub value: JsonValue,
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_test::{assert_de_tokens, assert_tokens, Token};

    #[test]
    fn test_aggregation_op() {
        let count = AggregationOp::Count;
        assert_tokens(&count, &[Token::Str("count")]);
        assert_de_tokens(&count, &[Token::Str("count")]);
    }

    #[test]
    fn test_filter_op() {
        let and = FilterOp::And;
        assert_tokens(&and, &[Token::Str("and")]);
        assert_de_tokens(&and, &[Token::Str("and")]);
    }
}
