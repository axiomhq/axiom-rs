//! The top-level client for the Axiom API.
use datasets::{
    AplOptions, AplQuery, AplQueryParams, AplQueryResult, Query, QueryOptions, QueryResult,
};
use std::env;
use std::fmt::Debug as FmtDebug;
use tracing::instrument;

use crate::{
    datasets,
    error::{Error, Result},
    http, is_personal_token, users,
};

/// Cloud URL is the URL for Axiom Cloud.
static CLOUD_URL: &str = "https://cloud.axiom.co";

/// The client is the entrypoint of the whole SDK.
///
/// You can create it using [`Client::builder`] or [`Client::new`].
///
/// # Examples
/// ```
/// use axiom_rs::{Client, Error};
///
/// fn main() -> Result<(), Error> {
///     // Create a new client and get the token and (if necesary) org id
///     // from the environment variables AXIOM_TOKEN and AXIOM_ORG_ID.
///     let client = Client::new()?;
///
///     // Set all available options. Unset options fall back to environment
///     // variables.
///     let client = Client::builder()
///         .with_token("my-token")
///         .with_org_id("my-org-id")
///         .build()?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    http_client: http::Client,

    url: String,
    pub datasets: datasets::Client,
    pub users: users::Client,
}

impl Client {
    /// Creates a new client. If you want to configure it, use [`Client::builder`].
    pub fn new() -> Result<Self> {
        Self::builder().build()
    }

    /// Create a new client using a builder.
    pub fn builder() -> Builder {
        Builder::new()
    }

    /// Get the url (cloned).
    #[doc(hidden)]
    pub fn url(&self) -> String {
        self.url.clone()
    }

    /// Get client version.
    pub async fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Executes the given query specified using the Axiom Processing Language (APL).
    #[instrument(skip(self, opts))]
    pub async fn query<S, O>(&self, apl: S, opts: O) -> Result<AplQueryResult>
    where
        S: Into<String> + FmtDebug,
        O: Into<Option<AplOptions>>,
    {
        let (req, query_params) = match opts.into() {
            Some(opts) => {
                let req = AplQuery {
                    apl: apl.into(),
                    start_time: opts.start_time,
                    end_time: opts.end_time,
                };

                let query_params = AplQueryParams {
                    no_cache: opts.no_cache,
                    save: opts.save,
                    format: opts.format,
                };

                (req, query_params)
            }
            None => (
                AplQuery {
                    apl: apl.into(),
                    ..Default::default()
                },
                AplQueryParams::default(),
            ),
        };

        let query_params = serde_qs::to_string(&query_params)?;
        let path = format!("/v1/datasets/_apl?{}", query_params);
        let res = self.http_client.post(path, &req).await?;

        let saved_query_id = res
            .headers()
            .get("X-Axiom-History-Query-Id")
            .map(|s| s.to_str())
            .transpose()
            .map_err(|_e| Error::InvalidQueryId)?
            .map(|s| s.to_string());

        let mut result = res.json::<AplQueryResult>().await?;
        result.saved_query_id = saved_query_id;

        Ok(result)
    }

    /// Execute the given query on the dataset identified by its id.
    #[instrument(skip(self, opts))]
    #[deprecated(
        since = "0.6.0",
        note = "The legacy query will be removed in future versions, use `apl_query` instead"
    )]
    pub async fn query_legacy<N, O>(
        &self,
        dataset_name: N,
        query: Query,
        opts: O,
    ) -> Result<QueryResult>
    where
        N: Into<String> + FmtDebug,
        O: Into<Option<QueryOptions>>,
    {
        let path = format!(
            "/v1/datasets/{}/query?{}",
            dataset_name.into(),
            &opts
                .into()
                .map(|opts| { serde_qs::to_string(&opts) })
                .unwrap_or_else(|| Ok(String::new()))?
        );
        let res = self.http_client.post(path, &query).await?;

        let saved_query_id = res
            .headers()
            .get("X-Axiom-History-Query-Id")
            .map(|s| s.to_str())
            .transpose()
            .map_err(|_e| Error::InvalidQueryId)?
            .map(|s| s.to_string());
        let mut result = res.json::<QueryResult>().await?;
        result.saved_query_id = saved_query_id;

        Ok(result)
    }
}

/// This builder is used to create a new client.
pub struct Builder {
    env_fallback: bool,
    url: Option<String>,
    token: Option<String>,
    org_id: Option<String>,
}

impl Builder {
    /// Create a new builder.
    fn new() -> Self {
        Self {
            env_fallback: true,
            url: None,
            token: None,
            org_id: None,
        }
    }

    /// Don't fall back to environment variables.
    pub fn no_env(mut self) -> Self {
        self.env_fallback = false;
        self
    }

    /// Add a token to the client. If this is not set, the token will be read
    /// from the environment variable `AXIOM_TOKEN`.
    pub fn with_token<S: Into<String>>(mut self, token: S) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Add an URL to the client. This is only meant for testing purposes, you
    /// don't need to set it.
    #[doc(hidden)]
    pub fn with_url<S: Into<String>>(mut self, url: S) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Add an organization ID to the client. If this is not set, the
    /// organization ID will be read from the environment variable `AXIOM_ORG_ID`.
    pub fn with_org_id<S: Into<String>>(mut self, org_id: S) -> Self {
        self.org_id = Some(org_id.into());
        self
    }

    /// Build the client.
    pub fn build(self) -> Result<Client> {
        let env_fallback = self.env_fallback;

        let mut token = self.token.unwrap_or_default();
        if token.is_empty() && env_fallback {
            token = env::var("AXIOM_TOKEN").unwrap_or_default();
        }
        if token.is_empty() {
            return Err(Error::MissingToken);
        }

        let mut url = self.url.unwrap_or_default();
        if url.is_empty() && env_fallback {
            url = env::var("AXIOM_URL").unwrap_or_default();
        }
        if url.is_empty() {
            url = CLOUD_URL.to_string();
        }

        let mut org_id = self.org_id.unwrap_or_default();
        if org_id.is_empty() && env_fallback {
            org_id = env::var("AXIOM_ORG_ID").unwrap_or_default();
        };

        // On Cloud you need an Org ID for Personal Tokens.
        if url == CLOUD_URL && org_id.is_empty() && is_personal_token(&token) {
            return Err(Error::MissingOrgId);
        }

        let http_client = http::Client::new(url.clone(), token, org_id)?;

        Ok(Client {
            http_client: http_client.clone(),
            url,
            datasets: datasets::Client::new(http_client.clone()),
            users: users::Client::new(http_client),
        })
    }
}
