//! Error type definitions.

use serde::Deserialize;
use std::fmt;
use thiserror::Error;

use crate::limits::Limits;

/// A `Result` alias where the `Err` case is `axiom::Error`.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for the Axiom client.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
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
    #[error("Http error: {0}")]
    /// HTTP error.
    Http(reqwest::Error),
    #[error(transparent)]
    /// Axion API error.
    Axiom(AxiomError),
    #[error("Query ID contains invisible characters (this is a server error)")]
    /// Query ID contains invisible characters (this is a server error).
    InvalidQueryId,
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
    /// Duration is out of range (can't be larger than i64::MAX milliseconds).
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
}

/// This is the manual implementation. We don't really care if the error is
/// permanent or transient at this stage so we just return Error::Http.
impl From<backoff::Error<reqwest::Error>> for Error {
    fn from(err: backoff::Error<reqwest::Error>) -> Self {
        match err {
            backoff::Error::Permanent(err) => Error::Http(err),
            backoff::Error::Transient {
                err,
                retry_after: _,
            } => Error::Http(err),
        }
    }
}

/// An error returned by the Axiom API.
#[derive(Deserialize, Debug)]
pub struct AxiomError {
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
}

impl AxiomError {
    pub(crate) fn new(
        status: u16,
        method: http::Method,
        path: String,
        message: Option<String>,
    ) -> Self {
        Self {
            status,
            method,
            path,
            message,
        }
    }
}

impl std::error::Error for AxiomError {}

impl fmt::Display for AxiomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(msg) = self.message.as_ref() {
            write!(
                f,
                "Received {} on {} {}: {}",
                self.status, self.method, self.path, msg
            )
        } else {
            write!(
                f,
                "Received {} on {} {})",
                self.method, self.path, self.status
            )
        }
    }
}
