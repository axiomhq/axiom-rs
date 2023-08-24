use backoff::future::retry;
use http::header;
use http::HeaderMap;
use serde::de::DeserializeOwned;
use std::time::Duration;
use url::Url;

use crate::{
    error::{AxiomError, Error, Result},
    http::{build_backoff, Body, USER_AGENT},
    limits::Limit,
};

/// Client is a wrapper around reqwest::Client which provides automatically
/// prepending the base url.
#[derive(Debug, Clone)]
pub(crate) struct Client {
    base_url: Url,
    inner: reqwest::Client,
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
        })
    }

    pub(crate) async fn execute<P, H>(
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

        let res = retry(build_backoff(), || async {
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
        .map(|res| Response::new(res, method, path.as_ref().to_string()))
        .map_err(Error::Http)?;

        Ok(res)
    }
}

pub(crate) struct Response {
    inner: reqwest::Response,
    method: http::Method,
    path: String,
    limits: Option<Limit>,
}

impl Response {
    pub(crate) fn new(inner: reqwest::Response, method: http::Method, path: String) -> Self {
        let limits = Limit::try_from(&inner);
        Self {
            inner,
            method,
            path,
            limits,
        }
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
            // Check if we hit some limits
            match self.limits {
                Some(Limit::Rate(scope, limits)) => {
                    return Err(Error::RateLimitExceeded { scope, limits });
                }
                Some(Limit::Query(limit)) => {
                    return Err(Error::QueryLimitExceeded(limit));
                }
                Some(Limit::Ingest(limit)) => {
                    return Err(Error::IngestLimitExceeded(limit));
                }
                None => {}
            }

            // Try to decode the error
            let e = match self.inner.json::<AxiomError>().await {
                Ok(mut e) => {
                    e.status = status.as_u16();
                    e.method = self.method;
                    e.path = self.path;
                    Error::Axiom(e)
                }
                Err(_e) => {
                    // Decoding failed, we still want an AxiomError
                    Error::Axiom(AxiomError::new(
                        status.as_u16(),
                        self.method,
                        self.path,
                        None,
                    ))
                }
            };
            return Err(e);
        }

        Ok(self)
    }

    pub(crate) fn get_header(&self, name: impl AsRef<str>) -> Option<&str> {
        self.inner
            .headers()
            .get(name.as_ref())
            .and_then(|name| name.to_str().ok())
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
            then.status(430)
                .json_body(json!({ "message": "ingest limit exceeded" }))
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

        match client.ingest("test", vec![json!({"foo": "bar"})]).await {
            Err(Error::IngestLimitExceeded(limits)) => {
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
            then.status(430)
                .json_body(json!({ "message": "query limit exceeded" }))
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

        match client.query("test | count", None).await {
            Err(Error::QueryLimitExceeded(limits)) => {
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
            Err(Error::RateLimitExceeded { scope, limits }) => {
                assert_eq!(scope, "user");
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
