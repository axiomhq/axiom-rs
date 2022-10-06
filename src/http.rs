#[cfg(feature = "async-std")]
use async_std::sync::Mutex;
use backoff::{future::retry, ExponentialBackoffBuilder};
use bytes::Bytes;
use http::header;
pub(crate) use http::HeaderMap;
use serde::{de::DeserializeOwned, Serialize};
use std::{env, sync::Arc, time::Duration};
#[cfg(feature = "tokio")]
use tokio::sync::Mutex;
use url::Url;

use crate::{
    error::{AxiomError, Error, Result},
    limits::{Limit, Limits},
};

static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

/// Client is a wrapper around reqwest::Client which provides automatically
/// prepending the base url.
#[derive(Debug, Clone)]
pub(crate) struct Client {
    base_url: Url,
    inner: reqwest::Client,
    ingest_limit: Arc<Mutex<Option<Limits>>>,
    query_limit: Arc<Mutex<Option<Limits>>>,
}

#[derive(Clone)]
pub(crate) enum Body {
    Empty,
    Json(serde_json::Value),
    Bytes(Bytes),
}

impl Client {
    /// Creates a new client.
    pub(crate) fn new<U, T, O>(base_url: U, token: T, org_id: O) -> Result<Self>
    where
        U: AsRef<str>,
        T: Into<String>,
        O: Into<Option<String>>,
    {
        let base_url = Url::parse(base_url.as_ref()).map_err(Error::InvalidUrl)?;
        let token = token.into();

        let mut default_headers = header::HeaderMap::new();
        let token_header_value = header::HeaderValue::from_str(&format!("Bearer {}", token))
            .map_err(|_e| Error::InvalidToken)?;
        default_headers.insert(header::AUTHORIZATION, token_header_value);
        if let Some(org_id) = org_id.into() {
            let org_id_header_value =
                header::HeaderValue::from_str(&org_id).map_err(|_e| Error::InvalidOrgId)?;
            default_headers.insert("X-Axiom-Org-Id", org_id_header_value);
        }

        let http_client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .default_headers(default_headers)
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(Error::HttpClientSetup)?;

        Ok(Self {
            base_url,
            inner: http_client,
            ingest_limit: Arc::new(Mutex::new(None)),
            query_limit: Arc::new(Mutex::new(None)),
        })
    }

    async fn execute<P, H>(
        &self,
        method: http::Method,
        path: P,
        body: Body,
        headers: H,
    ) -> Result<Response>
    where
        P: AsRef<str>,
        H: Into<Option<HeaderMap>>,
    {
        let url = self
            .base_url
            .join(path.as_ref().trim_start_matches('/'))
            .map_err(Error::InvalidUrl)?;

        let headers = headers.into();
        let backoff = ExponentialBackoffBuilder::new()
            .with_initial_interval(Duration::from_millis(500)) // first retry after 500ms
            .with_multiplier(2.0) // all following retries are twice as long as the previous one
            .with_max_elapsed_time(Some(Duration::from_secs(30))) // try up to 30s
            .build();

        let res = retry(backoff, || async {
            let mut req = self.inner.request(method.clone(), url.clone());
            if let Some(headers) = headers.clone() {
                req = req.headers(headers);
            }
            match body.clone() {
                Body::Empty => {}
                Body::Json(value) => req = req.json(&value),
                Body::Bytes(bytes) => req = req.body(bytes),
            }
            self.inner.execute(req.build()?).await.map_err(|e| {
                if let Some(status) = e.status() {
                    if status.is_client_error() {
                        // Don't retry 4XX
                        return backoff::Error::permanent(e);
                    }
                }

                backoff::Error::transient(e)
            })
        })
        .await
        .map(Response::new)
        .map_err(Error::Http)?;

        self.update_limits(&res.limits).await;

        Ok(res)
    }

    async fn update_limits(&self, limit: &Option<Limit>) {
        match limit {
            Some(Limit::Ingest(limit)) => {
                let mut ingest_limit = self.ingest_limit.lock().await;
                ingest_limit.replace(limit.clone());
            }
            Some(Limit::Query(limit)) => {
                let mut query_limit = self.query_limit.lock().await;
                query_limit.replace(limit.clone());
            }
            _ => {}
        };
    }

    pub(crate) async fn get<S>(&self, path: S) -> Result<Response>
    where
        S: AsRef<str>,
    {
        self.execute(http::Method::GET, path.as_ref(), Body::Empty, None)
            .await
    }

    pub(crate) async fn post<S, P>(&self, path: S, payload: P) -> Result<Response>
    where
        S: AsRef<str>,
        P: Serialize,
    {
        self.execute(
            http::Method::POST,
            path,
            Body::Json(serde_json::to_value(payload).map_err(Error::Serialize)?),
            None,
        )
        .await
    }

    pub(crate) async fn post_bytes<S, P, H>(
        &self,
        path: S,
        payload: P,
        headers: H,
    ) -> Result<Response>
    where
        S: AsRef<str>,
        P: Into<Bytes>,
        H: Into<Option<HeaderMap>>,
    {
        self.execute(
            http::Method::POST,
            path,
            Body::Bytes(payload.into()),
            headers,
        )
        .await
    }

    pub(crate) async fn put<S, P>(&self, path: S, payload: P) -> Result<Response>
    where
        S: AsRef<str>,
        P: Serialize,
    {
        self.execute(
            http::Method::PUT,
            path,
            Body::Json(serde_json::to_value(payload).map_err(Error::Serialize)?),
            None,
        )
        .await
    }

    pub(crate) async fn delete<S>(&self, path: S) -> Result<()>
    where
        S: AsRef<str>,
    {
        self.execute(http::Method::DELETE, path, Body::Empty, None)
            .await?;
        Ok(())
    }
}

pub(crate) struct Response {
    inner: reqwest::Response,
    limits: Option<Limit>,
}

impl Response {
    pub(crate) fn new(inner: reqwest::Response) -> Self {
        let limits = Limit::try_from(&inner);
        Self { inner, limits }
    }

    pub(crate) async fn json<T: DeserializeOwned>(self) -> Result<T> {
        self.check_error()
            .await?
            .inner
            .json::<T>()
            .await
            .map_err(Error::Deserialize)
    }

    pub(crate) async fn check_error(self) -> Result<Response> {
        let status = self.inner.status();
        if !status.is_success() {
            if status == http::StatusCode::TOO_MANY_REQUESTS {
                if let Some(limits) = self.limits {
                    return Err(Error::RateLimitExceeded(limits.into_inner()));
                }
            }

            let e = match self.inner.json::<AxiomError>().await {
                Ok(mut e) => {
                    e.status = status.as_u16();
                    Error::Axiom(e)
                }
                Err(_e) => {
                    // Decoding failed, we still want an AxiomError
                    Error::Axiom(AxiomError::new(status.as_u16(), None))
                }
            };
            return Err(e);
        }

        Ok(self)
    }

    pub(crate) fn headers(&self) -> &header::HeaderMap {
        self.inner.headers()
    }
}

impl From<reqwest::Response> for Response {
    fn from(inner: reqwest::Response) -> Self {
        Self::new(inner)
    }
}

impl From<Response> for reqwest::Response {
    fn from(res: Response) -> Self {
        res.inner
    }
}

#[cfg(test)]
mod test {
    use chrono::{Duration, Utc};
    use httpmock::prelude::*;
    use serde_json::json;

    use crate::{limits, Client, Error};

    #[tokio::test]
    async fn test_ingest_limit_exceeded() -> Result<(), Box<dyn std::error::Error>> {
        let expires_after = Duration::seconds(1);
        let tomorrow = Utc::now() + expires_after;

        let server = MockServer::start();
        let rate_mock = server.mock(|when, then| {
            when.method(POST).path("/v1/datasets/test/ingest");
            then.status(429)
                .json_body(json!({ "message": "rate limit exceeded" }))
                .header(limits::HEADER_INGEST_LIMIT, "42")
                .header(limits::HEADER_INGEST_REMAINING, "0")
                .header(
                    limits::HEADER_INGEST_RESET,
                    format!("{}", tomorrow.timestamp()),
                );
        });

        let client = Client::builder()
            .no_env()
            .with_url(server.base_url())
            .with_token("xapt-nope")
            .build()?;

        match client
            .datasets
            .ingest("test", vec![json!({"foo": "bar"})])
            .await
        {
            Err(Error::RateLimitExceeded(limits)) => {
                assert_eq!(limits.limit, 42);
                assert_eq!(limits.remaining, 0);
                assert_eq!(limits.reset.timestamp(), tomorrow.timestamp());
            }
            res => panic!("Expected ingest limit error, got {:?}", res),
        };

        rate_mock.assert_hits_async(1).await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_limit_exceeded() -> Result<(), Box<dyn std::error::Error>> {
        let expires_after = Duration::seconds(1);
        let tomorrow = Utc::now() + expires_after;

        let server = MockServer::start();
        let rate_mock = server.mock(|when, then| {
            when.method(POST).path("/v1/datasets/_apl");
            then.status(429)
                .json_body(json!({ "message": "rate limit exceeded" }))
                .header(limits::HEADER_QUERY_LIMIT, "42")
                .header(limits::HEADER_QUERY_REMAINING, "0")
                .header(
                    limits::HEADER_QUERY_RESET,
                    format!("{}", tomorrow.timestamp()),
                );
        });

        let client = Client::builder()
            .no_env()
            .with_url(server.base_url())
            .with_token("xapt-nope")
            .build()?;

        match client.datasets.apl_query("test | count", None).await {
            Err(Error::RateLimitExceeded(limits)) => {
                assert_eq!(limits.limit, 42);
                assert_eq!(limits.remaining, 0);
                assert_eq!(limits.reset.timestamp(), tomorrow.timestamp());
            }
            res => panic!("Expected ingest limit error, got {:?}", res),
        };

        rate_mock.assert_hits_async(1).await;
        Ok(())
    }

    #[tokio::test]
    async fn test_rate_limit_exceeded() -> Result<(), Box<dyn std::error::Error>> {
        let expires_after = Duration::seconds(1);
        let tomorrow = Utc::now() + expires_after;

        let server = MockServer::start();
        let rate_mock = server.mock(|when, then| {
            when.method(GET).path("/v1/datasets");
            then.status(429)
                .json_body(json!({ "message": "rate limit exceeded" }))
                .header(limits::HEADER_RATE_SCOPE, "user")
                .header(limits::HEADER_RATE_LIMIT, "42")
                .header(limits::HEADER_RATE_REMAINING, "0")
                .header(
                    limits::HEADER_RATE_RESET,
                    format!("{}", tomorrow.timestamp()),
                );
        });

        let client = Client::builder()
            .no_env()
            .with_url(server.base_url())
            .with_token("xapt-nope")
            .build()?;

        match client.datasets.list().await {
            Err(Error::RateLimitExceeded(limits)) => {
                assert_eq!(limits.limit, 42);
                assert_eq!(limits.remaining, 0);
                assert_eq!(limits.reset.timestamp(), tomorrow.timestamp());
            }
            res => panic!("Expected ingest limit error, got {:?}", res),
        };

        rate_mock.assert_hits_async(1).await;
        Ok(())
    }
}
