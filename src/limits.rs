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
pub(crate) enum Error {
    #[error("Invalid limit header")]
    InvalidLimitHeader,
    #[error("Invalid remaining header")]
    InvalidRemainingHeader,
    #[error("Invalid reset header")]
    InvalidResetHeader,
}

#[derive(Debug, Clone)]
pub(crate) enum Limit {
    Query(Limits),
    Ingest(Limits),
    Rate(String, Limits),
}

impl Limit {
    pub(crate) fn into_inner(self) -> Limits {
        match self {
            Limit::Query(l) => l,
            Limit::Ingest(l) => l,
            Limit::Rate(_, l) => l,
        }
    }

    pub(crate) fn try_from(response: &reqwest::Response) -> Option<Self> {
        let path = response.url().path();
        if path.ends_with("/ingest") {
            Limits::from_headers(
                response.headers(),
                HEADER_INGEST_LIMIT,
                HEADER_INGEST_REMAINING,
                HEADER_INGEST_RESET,
            )
            .map(|limits| Limit::Ingest(limits))
            .ok()
        } else if path.ends_with("/query") || path.ends_with("/_apl") {
            Limits::from_headers(
                response.headers(),
                HEADER_QUERY_LIMIT,
                HEADER_QUERY_REMAINING,
                HEADER_QUERY_RESET,
            )
            .map(|limits| Limit::Query(limits))
            .ok()
        } else {
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
        return self.remaining == 0 && self.reset > Utc::now();
    }

    fn from_headers(
        headers: &header::HeaderMap,
        header_limit: &str,
        header_remaining: &str,
        header_reset: &str,
    ) -> Result<Self, Error> {
        Ok(Limits {
            limit: headers
                .get(header_limit)
                .and_then(|limit| Some(limit.to_str().unwrap()))
                .and_then(|limit| Some(limit.parse::<u64>().unwrap()))
                .ok_or_else(|| Error::InvalidLimitHeader)?,
            remaining: headers
                .get(header_remaining)
                .and_then(|limit| limit.to_str().ok())
                .and_then(|limit| limit.parse::<u64>().ok())
                .ok_or_else(|| Error::InvalidRemainingHeader)?,
            reset: headers
                .get(header_reset)
                .and_then(|limit| limit.to_str().ok())
                .and_then(|limit| limit.parse::<i64>().ok())
                .map(|limit| Utc.timestamp(limit, 0))
                .ok_or_else(|| Error::InvalidResetHeader)?,
        })
    }
}
