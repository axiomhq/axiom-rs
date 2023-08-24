//! Rate-limit type definitions.

use chrono::{DateTime, TimeZone, Utc};
use http::header;
use std::fmt::Display;
use thiserror::Error;

pub(crate) const HEADER_QUERY_LIMIT: &str = "X-QueryLimit-Limit";
pub(crate) const HEADER_QUERY_REMAINING: &str = "X-QueryLimit-Remaining";
pub(crate) const HEADER_QUERY_RESET: &str = "X-QueryLimit-Reset";

pub(crate) const HEADER_INGEST_LIMIT: &str = "X-IngestLimit-Limit";
pub(crate) const HEADER_INGEST_REMAINING: &str = "X-IngestLimit-Remaining";
pub(crate) const HEADER_INGEST_RESET: &str = "X-IngestLimit-Reset";

pub(crate) const HEADER_RATE_SCOPE: &str = "X-RateLimit-Scope";
pub(crate) const HEADER_RATE_LIMIT: &str = "X-RateLimit-Limit";
pub(crate) const HEADER_RATE_REMAINING: &str = "X-RateLimit-Remaining";
pub(crate) const HEADER_RATE_RESET: &str = "X-RateLimit-Reset";

#[derive(Error, Debug)]
pub(crate) enum InvalidHeaderError {
    #[error("Invalid limit header")]
    Limit,
    #[error("Invalid remaining header")]
    Remaining,
    #[error("Invalid reset header")]
    Reset,
}

#[derive(Debug, Clone)]
pub(crate) enum Limit {
    Ingest(Limits),
    Query(Limits),
    Rate(String, Limits),
}

impl Limit {
    #[cfg(not(feature = "blocking"))]
    pub(crate) fn try_from(response: &reqwest::Response) -> Option<Self> {
        match response.status().as_u16() {
            429 => {
                // Rate limit
                let scope = response
                    .headers()
                    .get(HEADER_RATE_SCOPE)
                    .and_then(|limit| limit.to_str().ok());
                let limits = Limits::from_headers(
                    response.headers(),
                    HEADER_RATE_LIMIT,
                    HEADER_RATE_REMAINING,
                    HEADER_RATE_RESET,
                )
                .ok();

                scope
                    .zip(limits)
                    .map(|(scope, limits)| Limit::Rate(scope.to_string(), limits))
            }
            430 => {
                // Query or ingest limit
                let query_limit = Limits::from_headers(
                    response.headers(),
                    HEADER_QUERY_LIMIT,
                    HEADER_QUERY_REMAINING,
                    HEADER_QUERY_RESET,
                )
                .map(Limit::Query)
                .ok();
                let ingest_limit = Limits::from_headers(
                    response.headers(),
                    HEADER_INGEST_LIMIT,
                    HEADER_INGEST_REMAINING,
                    HEADER_INGEST_RESET,
                )
                .map(Limit::Ingest)
                .ok();

                // Can't have both
                query_limit.or(ingest_limit)
            }
            _ => None,
        }
    }

    #[cfg(feature = "blocking")]
    pub(crate) fn try_from(response: &ureq::Response) -> Option<Self> {
        match response.status() {
            429 => {
                // Rate limit
                let scope = response.header(HEADER_RATE_SCOPE);
                let limits = Limits::from_headers(
                    response,
                    HEADER_RATE_LIMIT,
                    HEADER_RATE_REMAINING,
                    HEADER_RATE_RESET,
                )
                .ok();

                scope
                    .zip(limits)
                    .map(|(scope, limits)| Limit::Rate(scope.to_string(), limits))
            }
            430 => {
                // Query or ingest limit
                let query_limit = Limits::from_headers(
                    response,
                    HEADER_QUERY_LIMIT,
                    HEADER_QUERY_REMAINING,
                    HEADER_QUERY_RESET,
                )
                .map(Limit::Query)
                .ok();
                let ingest_limit = Limits::from_headers(
                    response,
                    HEADER_INGEST_LIMIT,
                    HEADER_INGEST_REMAINING,
                    HEADER_INGEST_RESET,
                )
                .map(Limit::Ingest)
                .ok();

                // Can't have both
                query_limit.or(ingest_limit)
            }
            _ => None,
        }
    }
}

/// Rate-limit information.
#[derive(Debug, Clone)]
pub struct Limits {
    /// The maximum limit a client is limited to for a specified time window
    /// which resets at the time indicated by `reset`.
    pub limit: u64,
    /// The remaining count towards the maximum limit.
    pub remaining: u64,
    /// The time at which the current limit time window will reset.
    pub reset: DateTime<Utc>,
}

impl Display for Limits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{} remaining until {}",
            self.remaining, self.limit, self.reset
        )
    }
}

impl Limits {
    pub fn is_exceeded(&self) -> bool {
        self.remaining == 0 && self.reset > Utc::now()
    }

    #[cfg(not(feature = "blocking"))]
    fn from_headers(
        headers: &header::HeaderMap,
        header_limit: &str,
        header_remaining: &str,
        header_reset: &str,
    ) -> Result<Self, InvalidHeaderError> {
        Ok(Limits {
            limit: headers
                .get(header_limit)
                .and_then(|limit| limit.to_str().ok())
                .and_then(|limit| limit.parse::<u64>().ok())
                .ok_or(InvalidHeaderError::Limit)?,
            remaining: headers
                .get(header_remaining)
                .and_then(|limit| limit.to_str().ok())
                .and_then(|limit| limit.parse::<u64>().ok())
                .ok_or(InvalidHeaderError::Remaining)?,
            reset: headers
                .get(header_reset)
                .and_then(|limit| limit.to_str().ok())
                .and_then(|limit| limit.parse::<i64>().ok())
                .and_then(|limit| Utc.timestamp_opt(limit, 0).single())
                .ok_or(InvalidHeaderError::Reset)?,
        })
    }

    #[cfg(feature = "blocking")]
    fn from_headers(
        response: &ureq::Response,
        header_limit: &str,
        header_remaining: &str,
        header_reset: &str,
    ) -> Result<Self, InvalidHeaderError> {
        Ok(Limits {
            limit: response
                .header(header_limit)
                .and_then(|limit| limit.parse::<u64>().ok())
                .ok_or(InvalidHeaderError::Limit)?,
            remaining: response
                .header(header_remaining)
                .and_then(|limit| limit.parse::<u64>().ok())
                .ok_or(InvalidHeaderError::Remaining)?,
            reset: response
                .header(header_reset)
                .and_then(|limit| limit.parse::<i64>().ok())
                .and_then(|limit| Utc.timestamp_opt(limit, 0).single())
                .ok_or(InvalidHeaderError::Reset)?,
        })
    }
}
