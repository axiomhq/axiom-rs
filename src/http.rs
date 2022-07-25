use backoff::{future::retry, ExponentialBackoffBuilder};
use bytes::Bytes;
pub use http::HeaderMap;
use reqwest::header;
use serde::{de::DeserializeOwned, Serialize};
use std::{env, future::Future, result::Result as StdResult, time::Duration};

use crate::error::{AxiomError, Error, Result};

static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

/// Client is a wrapper around reqwest::Client which provides automatically
/// prepending the base url.
#[derive(Clone)]
pub(crate) struct Client {
    base_url: String,
    inner: reqwest::Client,
}

impl Client {
    /// Creates a new client.
    pub(crate) fn new<S: Into<String>>(base_url: S, token: S, org_id: Option<S>) -> Result<Self> {
        let base_url = format!("{}/api/v1", base_url.into());
        let token = token.into();

        let mut default_headers = header::HeaderMap::new();
        let token_header_value = header::HeaderValue::from_str(&format!("Bearer {}", token))
            .map_err(|_e| Error::InvalidToken)?;
        default_headers.insert(header::AUTHORIZATION, token_header_value);
        if let Some(org_id) = org_id {
            let org_id_header_value =
                header::HeaderValue::from_str(&org_id.into()).map_err(|_e| Error::InvalidOrgId)?;
            default_headers.insert("X-Axiom-Org-Id", org_id_header_value);
        }

        let http_client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .default_headers(default_headers)
            .build()
            .map_err(Error::HttpClientSetup)?;

        Ok(Self {
            base_url,
            inner: http_client,
        })
    }

    async fn retry<F, Fut>(f: F) -> Result<Response>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = StdResult<reqwest::Response, reqwest::Error>>,
    {
        let backoff = ExponentialBackoffBuilder::new()
            .with_initial_interval(Duration::from_millis(500)) // first retry after 500ms
            .with_multiplier(2.0) // all following retries are twice as long as the previous one
            .with_max_elapsed_time(Some(Duration::from_secs(30))) // try up to 30s
            .build();
        retry(backoff, || async {
            f().await.map_err(|e| {
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
        .map_err(Error::Http)
    }

    pub(crate) async fn get<S>(&self, path: S) -> Result<Response>
    where
        S: Into<String>,
    {
        let url = format!("{}{}", self.base_url, path.into());
        Self::retry(|| async { self.inner.get(&url).send().await }).await
    }

    pub(crate) async fn post<S, P>(&self, path: S, payload: P) -> Result<Response>
    where
        S: Into<String>,
        P: Serialize,
    {
        let url = format!("{}{}", self.base_url, path.into());
        Self::retry(|| async { self.inner.post(&url).json(&payload).send().await }).await
    }

    pub(crate) async fn post_bytes<S, P, H>(
        &self,
        path: S,
        payload: P,
        headers: H,
    ) -> Result<Response>
    where
        S: Into<String>,
        P: Into<Bytes>,
        H: Into<Option<HeaderMap>>,
    {
        let url = format!("{}{}", self.base_url, path.into());
        let payload = payload.into();
        let headers = headers.into().unwrap_or(HeaderMap::new());
        Self::retry(|| async {
            self.inner
                .post(&url)
                .body(payload.clone())
                .headers(headers.clone())
                .send()
                .await
        })
        .await
    }

    pub(crate) async fn put<S, P>(&self, path: S, payload: P) -> Result<Response>
    where
        S: Into<String>,
        P: Serialize,
    {
        let url = format!("{}{}", self.base_url, path.into());
        Self::retry(|| async { self.inner.put(&url).json(&payload).send().await }).await
    }

    pub(crate) async fn delete<S>(&self, path: S) -> Result<()>
    where
        S: Into<String>,
    {
        let url = format!("{}{}", self.base_url, path.into());
        Self::retry(|| async { self.inner.delete(&url).send().await })
            .await?
            .check_error()
            .await?;
        Ok(())
    }
}

pub(crate) struct Response {
    inner: reqwest::Response,
}

impl Response {
    pub(crate) fn new(inner: reqwest::Response) -> Self {
        Self { inner }
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
