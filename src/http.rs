use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};
use bytes::Bytes;
pub(crate) use http::HeaderMap;
use maybe_async::maybe_async;
use serde::Serialize;
use std::time::Duration;

use crate::error::{Error, Result};
#[cfg(not(feature = "blocking"))]
use crate::http_async::{Client as ClientImpl, Response as ResponseImpl};
#[cfg(feature = "blocking")]
use crate::http_blocking::{Client as ClientImpl, Response as ResponseImpl};

pub(crate) static USER_AGENT: &str =
    concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Clone)]
pub(crate) enum Body {
    Empty,
    Json(serde_json::Value),
    Bytes(Bytes),
}

pub(crate) fn build_backoff() -> ExponentialBackoff {
    ExponentialBackoffBuilder::new()
        .with_initial_interval(Duration::from_millis(500)) // first retry after 500ms
        .with_multiplier(2.0) // all following retries are twice as long as the previous one
        .with_max_elapsed_time(Some(Duration::from_secs(30))) // try up to 30s
        .build()
}

#[derive(Debug, Clone)]
pub(crate) struct Client {
    inner: ClientImpl,
}

impl Client {
    pub(crate) fn new<U, T, O>(base_url: U, token: T, org_id: O) -> Result<Self>
    where
        U: AsRef<str>,
        T: Into<String>,
        O: Into<Option<String>>,
    {
        Ok(Self {
            inner: ClientImpl::new(base_url, token, org_id)?,
        })
    }

    #[maybe_async]
    pub(crate) async fn get<S>(&self, path: S) -> Result<ResponseImpl>
    where
        S: AsRef<str>,
    {
        self.inner
            .execute(http::Method::GET, path.as_ref(), Body::Empty, None)
            .await
    }

    #[maybe_async]
    pub(crate) async fn post<S, P>(&self, path: S, payload: P) -> Result<ResponseImpl>
    where
        S: AsRef<str>,
        P: Serialize,
    {
        self.inner
            .execute(
                http::Method::POST,
                path,
                Body::Json(serde_json::to_value(payload).map_err(Error::Serialize)?),
                None,
            )
            .await
    }

    #[maybe_async]
    pub(crate) async fn post_bytes<S, P, H>(
        &self,
        path: S,
        payload: P,
        headers: H,
    ) -> Result<ResponseImpl>
    where
        S: AsRef<str>,
        P: Into<Bytes>,
        H: Into<Option<HeaderMap>>,
    {
        self.inner
            .execute(
                http::Method::POST,
                path,
                Body::Bytes(payload.into()),
                headers,
            )
            .await
    }

    #[maybe_async]
    pub(crate) async fn put<S, P>(&self, path: S, payload: P) -> Result<ResponseImpl>
    where
        S: AsRef<str>,
        P: Serialize,
    {
        self.inner
            .execute(
                http::Method::PUT,
                path,
                Body::Json(serde_json::to_value(payload).map_err(Error::Serialize)?),
                None,
            )
            .await
    }

    #[maybe_async]
    pub(crate) async fn delete<S>(&self, path: S) -> Result<()>
    where
        S: AsRef<str>,
    {
        self.inner
            .execute(http::Method::DELETE, path, Body::Empty, None)
            .await?;
        Ok(())
    }
}
