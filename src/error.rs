//! Error type definitions.

use serde::Deserialize;
use std::fmt;

use crate::limits::Limits;

/// A `Result` alias where the `Err` case is `axiom::Error`.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for the Axiom client.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("Invalid time order")]
    /// Invalid time
    InvalidTimeOrder,
    #[error("Empty update")]
    /// Empty update.
    EmptyUpdate,
    #[error("Empty datasets")]
    /// Empty datasets.
    EmptyDatasets,
    #[error("Empty type")]
    /// Empty type.
    EmptyType,
    #[error("Missing token")]
    /// Missing token.
    MissingToken,
    #[error("Missing Org ID for Personal Access Token")]
    /// Missing Org ID for Personal Access Token.
    MissingOrgId,
    #[error("Invalid token (make sure there are no invalid characters)")]
    /// Invalid token.
    InvalidToken,
    #[error("Invalid Org ID (make sure there are no invalid characters)")]
    /// Invalid Org ID.
    InvalidOrgId,
    #[error("Failed to setup HTTP client: {0}")]
    /// Failed to setup HTTP client.
    HttpClientSetup(reqwest::Error),
    #[error("Failed to deserialize response: {0}")]
    /// Failed to deserialize response.
    Deserialize(reqwest::Error),
    #[error(
        "Failed to decode response body from {method} {path}: {source}\nBody snippet: {body}"
    )]
    /// Failed to deserialize a 2xx response body. Carries a snippet of the
    /// raw body so the caller (and end user) can see what shape the server
    /// actually returned — the common cause is a `MetricsQueryResponse`
    /// (or similar) gaining a field, or the wrong `Accept` header pulling
    /// back a different variant.
    DeserializeBody {
        /// HTTP method that produced the response.
        method: http::Method,
        /// Request path that produced the response.
        path: String,
        /// Underlying serde error.
        #[source]
        source: serde_json::Error,
        /// Snippet of the raw response body (truncated).
        body: String,
    },
    #[error("Http error: {0}")]
    /// HTTP error.
    Http(reqwest::Error),
    #[error(transparent)]
    /// Axion API error.
    Axiom(Axiom),
    #[error("Query ID contains invisible characters (this is a server error)")]
    /// Query ID contains invisible characters (this is a server error).
    InvalidQueryId,
    #[error("Trace ID contains invisible characters (this is a server error)")]
    /// Query ID contains invisible characters (this is a server error).
    InvalidTraceId,
    #[error(transparent)]
    /// Invalid Query Parameters.
    InvalidParams(#[from] serde_qs::Error),
    #[error(transparent)]
    /// Invalid JSON.
    Serialize(#[from] serde_json::Error),
    #[error("Failed to encode payload: {0}")]
    /// Failed to encode payload.
    Encoding(std::io::Error),
    #[error("Duration is out of range (can't be larger than i64::MAX milliseconds)")]
    /// Duration is out of range (can't be larger than `i64::MAX` milliseconds).
    DurationOutOfRange,
    #[cfg(feature = "tokio")]
    #[error("Failed to join thread: {0}")]
    /// Failed to join thread.
    JoinError(tokio::task::JoinError),
    #[error("Rate limit exceeded for the {scope} scope: {limits}")]
    /// Rate limit exceeded.
    RateLimitExceeded {
        /// The scope of the rate limit.
        scope: String,
        /// The rate limit.
        limits: Limits,
    },
    #[error("Query limit exceeded: {0}")]
    /// Query limit exceeded.
    QueryLimitExceeded(Limits),
    #[error("Ingest limit exceeded: {0}")]
    /// Ingest limit exceeded.
    IngestLimitExceeded(Limits),
    #[error("Invalid URL: {0}")]
    /// Invalid URL.
    InvalidUrl(url::ParseError),
    #[error("Error in ingest stream: {0}")]
    /// Error in ingest stream.
    IngestStreamError(Box<dyn std::error::Error + Send + Sync>),
    #[error("Invalid content type: {0}")]
    /// Invalid content type.
    InvalidContentType(String),
    #[error("Invalid content encoding: {0}")]
    /// Invalid content encoding.
    InvalidContentEncoding(String),
    #[error(
        "Personal tokens are not supported for edge endpoints. Use an API token (xaat-) instead."
    )]
    /// Personal tokens are not supported for edge endpoints.
    PersonalTokenNotSupportedForEdge,
    #[error(
        "Dashboard version mismatch on uid {uid:?}: server is at version {current}. \
         Reload the dashboard or retry with `overwrite = true`."
    )]
    /// The server's optimistic version check rejected a dashboard write.
    /// Carries the server's current version so the caller can show a useful
    /// diff or reload prompt.
    DashboardVersionConflict {
        /// `uid` of the dashboard the conflict was reported for.
        uid: String,
        /// Current version on the server.
        current: i64,
    },
}

/// This is the manual implementation. We don't really care if the error is
/// permanent or transient at this stage so we just return `Error::Http`.
impl From<backoff::Error<reqwest::Error>> for Error {
    fn from(err: backoff::Error<reqwest::Error>) -> Self {
        match err {
            backoff::Error::Permanent(err)
            | backoff::Error::Transient {
                err,
                retry_after: _,
            } => Error::Http(err),
        }
    }
}

/// An error returned by the Axiom API.
#[derive(Deserialize, Debug)]
pub struct Axiom {
    #[serde(skip)]
    /// The HTTP status code.
    pub status: u16,
    #[serde(skip)]
    /// The HTTP method.
    pub method: http::Method,
    #[serde(skip)]
    /// The path that was requested.
    pub path: String,
    /// The error message.
    pub message: Option<String>,
    /// The trace id.
    #[serde(skip)]
    pub trace_id: Option<String>,
}

impl Axiom {
    pub(crate) fn new(
        status: u16,
        method: http::Method,
        path: String,
        message: Option<String>,
        trace_id: Option<String>,
    ) -> Self {
        Self {
            status,
            method,
            path,
            message,
            trace_id,
        }
    }
}

impl std::error::Error for Axiom {}

impl fmt::Display for Axiom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.message.as_ref(), self.trace_id.as_ref()) {
            (Some(msg), Some(trace_id)) => {
                write!(
                    f,
                    "Received {} on {} {}: {} (trace id: {})",
                    self.status, self.method, self.path, msg, trace_id
                )
            }
            (Some(msg), None) => {
                write!(
                    f,
                    "Received {} on {} {}: {}",
                    self.status, self.method, self.path, msg
                )
            }
            (None, Some(trace_id)) => {
                write!(
                    f,
                    "Received {} on {} {} (trace id: {})",
                    self.status, self.method, self.path, trace_id
                )
            }
            (None, None) => {
                write!(
                    f,
                    "Received {} on {} {})",
                    self.method, self.path, self.status
                )
            }
        }
    }
}
