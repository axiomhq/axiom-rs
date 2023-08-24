use backoff::retry;
use http::HeaderMap;
use serde::de::DeserializeOwned;
use std::time::Duration;
use ureq::{Agent, Middleware, MiddlewareNext, Request};
use url::Url;

use crate::{
    error::{AxiomError, Error},
    http::{build_backoff, Body, USER_AGENT},
    limits::Limit,
};

#[derive(Debug, Clone)]
pub(crate) struct Client {
    agent: Agent,
    base_url: Url,
}

impl Client {
    pub(crate) fn new(
        base_url: impl AsRef<str>,
        token: impl Into<String>,
        org_id: impl Into<Option<String>>,
    ) -> Result<Self, Error> {
        let base_url = Url::parse(base_url.as_ref()).map_err(Error::InvalidUrl)?;
        Ok(Self {
            agent: ureq::AgentBuilder::new()
                .user_agent(USER_AGENT)
                .middleware(TokenMiddleware::new(token, org_id))
                .timeout(Duration::from_secs(10))
                .build(),
            base_url,
        })
    }

    pub(crate) fn execute<P, H>(
        &self,
        method: http::Method,
        path: P,
        body: Body,
        headers: H,
    ) -> Result<Response, Error>
    where
        P: AsRef<str>,
        H: Into<Option<HeaderMap>>,
    {
        let url = self
            .base_url
            .join(path.as_ref())
            .map_err(Error::InvalidUrl)?;

        let mut req = self.agent.request_url(method.as_str(), &url);
        if let Some(headers) = headers.into() {
            for (key, value) in headers {
                if let Some(name) = key {
                    if let Some(value) = value.to_str().ok() {
                        req = req.set(name.as_str(), value);
                    }
                }
            }
        }

        let res = retry(build_backoff(), || {
            match &body {
                Body::Empty => req.clone().call(),
                Body::Json(json) => req.clone().send_json(json),
                Body::Bytes(bytes) => req.clone().send_bytes(&bytes),
            }
            .map_err(|e| match e {
                ureq::Error::Status(status, _) => {
                    if status >= 400 && status < 500 {
                        // Don't retry 4XX
                        backoff::Error::permanent(e)
                    } else {
                        backoff::Error::transient(e)
                    }
                }
                ureq::Error::Transport(_) => backoff::Error::transient(e),
            })
        })?;

        Ok(Response::new(res, method, path.as_ref().to_string()))
    }
}

struct TokenMiddleware {
    token: String,
    org_id: Option<String>,
}

impl TokenMiddleware {
    fn new(token: impl Into<String>, org_id: impl Into<Option<String>>) -> Self {
        Self {
            token: token.into(),
            org_id: org_id.into(),
        }
    }
}

impl Middleware for TokenMiddleware {
    fn handle(
        &self,
        request: Request,
        next: MiddlewareNext,
    ) -> Result<ureq::Response, ureq::Error> {
        let req = request.set("Authorization", &format!("Bearer {}", self.token));
        let req = if let Some(org_id) = &self.org_id {
            req.set("X-Axiom-Org-Id", org_id)
        } else {
            req
        };
        next.handle(req)
    }
}

pub(crate) struct Response {
    inner: ureq::Response,
    method: http::Method,
    path: String,
    limits: Option<Limit>,
}

impl Response {
    pub(crate) fn new(inner: ureq::Response, method: http::Method, path: String) -> Self {
        let limits = Limit::try_from(&inner);
        Self {
            inner,
            method,
            path,
            limits,
        }
    }

    pub(crate) fn json<T: DeserializeOwned>(self) -> Result<T, Error> {
        self.check_error()?
            .inner
            .into_json::<T>()
            .map_err(Error::Deserialize)
    }

    pub(crate) fn check_error(self) -> Result<Response, Error> {
        let status = self.inner.status();
        if status < 200 || status > 299 {
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
            let e = match self.inner.into_json::<AxiomError>() {
                Ok(mut e) => {
                    e.status = status;
                    e.method = self.method;
                    e.path = self.path;
                    Error::Axiom(e)
                }
                Err(_e) => {
                    // Decoding failed, we still want an AxiomError
                    Error::Axiom(AxiomError::new(status, self.method, self.path, None))
                }
            };
            return Err(e);
        }

        Ok(self)
    }

    pub(crate) fn get_header(&self, name: impl AsRef<str>) -> Option<&str> {
        self.inner.header(name.as_ref())
    }
}
