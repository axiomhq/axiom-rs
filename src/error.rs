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
    MissingToken,
    #[error("Missing Org ID for Personal Access Token")]
    MissingOrgId,
    #[error("Invalid token (make sure there are no invalid characters)")]
    InvalidToken,
    #[error("Invalid Org ID (make sure there are no invalid characters)")]
    InvalidOrgId,
    #[error("Failed to setup HTTP client: {0}")]
    HttpClientSetup(reqwest::Error),
    #[error("Failed to deserialize response: {0}")]
    Deserialize(reqwest::Error),
    #[error("Http error: {0}")]
    Http(reqwest::Error),
    #[error(transparent)]
    Axiom(AxiomError),
    #[error("Query ID contains invisible characters (this is a server error)")]
    InvalidQueryId,
    #[error(transparent)]
    InvalidParams(#[from] serde_qs::Error),
    #[error(transparent)]
    Serialize(#[from] serde_json::Error),
    #[error("Failed to encode payload: {0}")]
    Encoding(std::io::Error),
    #[error("Duration is out of range (can't be larger than i64::MAX milliseconds)")]
    DurationOutOfRange,
    #[cfg(feature = "tokio")]
    #[error("Failed to join thread: {0}")]
    JoinError(tokio::task::JoinError),
    #[error("Rate limit exceeded for the {scope} scope: {limits}")]
    RateLimitExceeded { scope: String, limits: Limits },
    #[error("Query limit exceeded: {0}")]
    QueryLimitExceeded(Limits),
    #[error("Ingest limit exceeded: {0}")]
    IngestLimitExceeded(Limits),
    #[error("Invalid URL: {0}")]
    InvalidUrl(url::ParseError),
    #[error("Error in ingest stream: {0}")]
    IngestStreamError(Box<dyn std::error::Error + Send + Sync>),
    #[error("Invalid content type: {0}")]
    InvalidContentType(String),
    #[error("Invalid content encoding: {0}")]
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
    pub status: u16,
    #[serde(skip)]
    pub method: http::Method,
    #[serde(skip)]
    pub path: String,
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
