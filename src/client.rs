//! The top-level client for the Axiom API.
#[cfg(feature = "async-std")]
use async_std::task::spawn_blocking;
use bytes::Bytes;
use flate2::{write::GzEncoder, Compression};
use futures::Stream;
use reqwest::header;
use serde::Serialize;
use std::{
    env, fmt::Debug as FmtDebug, io::Write, result::Result as StdResult,
    time::Duration as StdDuration,
};
#[cfg(feature = "tokio")]
use tokio::task::spawn_blocking;
use tokio_stream::StreamExt;
use tracing::instrument;

use crate::{
    annotations,
    datasets::{
        self, ContentEncoding, ContentType, IngestStatus, Query, QueryOptions, QueryParams,
        QueryResult,
    },
    error::{Error, Result},
    http::{self, HeaderMap},
    is_personal_token, users,
};

/// API URL is the URL for the Axiom Cloud API.
static API_URL: &str = "https://api.axiom.co";

/// Default edge URL for Axiom Cloud (US East 1).
static DEFAULT_EDGE_URL: &str = "https://us-east-1.aws.edge.axiom.co";

/// Request options that can be passed to some handlers.
#[derive(Debug, Default)]
pub struct RequestOptions {
    /// Additional headers for the request.
    pub additional_headers: HeaderMap,
}

/// The client is the entrypoint of the whole SDK.
///
/// You can create it using [`Client::builder`] or [`Client::new`].
///
/// # Examples
/// ```no_run
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
///     // Use edge ingestion for a specific region.
///     let client = Client::builder()
///         .with_token("my-token")
///         .with_region("eu-central-1.aws.edge.axiom.co")
///         .build()?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    /// HTTP client for API operations (datasets, users, annotations).
    api_http: http::Client,
    /// HTTP client for ingest and query operations (may use edge endpoint).
    ingest_http: http::Client,
    /// The API URL.
    api_url: String,
    /// The ingest/query URL (may be edge endpoint).
    ingest_url: String,
    /// Whether the ingest URL is an edge endpoint (uses different path format).
    uses_edge: bool,
}

impl Client {
    /// Creates a new client. If you want to configure it, use [`Client::builder`].
    ///
    /// # Errors
    /// If the client can not be created
    pub fn new() -> Result<Self> {
        Self::builder().build()
    }

    /// Create a new client using a builder.
    #[must_use]
    pub fn builder() -> Builder {
        Builder::new()
    }

    /// Dataset API
    #[must_use]
    pub fn datasets(&self) -> datasets::Client<'_> {
        datasets::Client::new(&self.api_http)
    }

    /// Users API
    #[must_use]
    pub fn users(&self) -> users::Client<'_> {
        users::Client::new(&self.api_http)
    }

    /// Annotations API
    #[must_use]
    pub fn annotations(&self) -> annotations::Client<'_> {
        annotations::Client::new(&self.api_http)
    }

    /// Get the API url
    #[doc(hidden)]
    #[must_use]
    pub fn api_url(&self) -> &str {
        &self.api_url
    }

    /// Get the ingest/query url
    #[doc(hidden)]
    #[must_use]
    pub fn ingest_url(&self) -> &str {
        &self.ingest_url
    }

    /// Returns true if the client is configured to use an edge endpoint.
    #[must_use]
    pub fn uses_edge(&self) -> bool {
        self.uses_edge
    }

    /// Get client version.
    #[must_use]
    pub fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Executes the given query specified using the Axiom Processing Language (APL).
    /// To learn more about APL, see the APL documentation at
    /// <https://www.axiom.co/docs/apl/introduction>.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request or JSON deserializing fails.
    #[instrument(skip(self, opts))]
    pub async fn query<S, O>(&self, apl: &S, opts: O) -> Result<QueryResult>
    where
        S: ToString + FmtDebug + ?Sized,
        O: Into<Option<QueryOptions>>,
    {
        let opts: QueryOptions = opts.into().unwrap_or_default();
        let query_params = QueryParams::from(&opts);
        let req = Query::new(apl, opts);

        let query_params = serde_qs::to_string(&query_params)?;
        let path = format!("/v1/datasets/_apl?{query_params}");
        // Query uses ingest_http as it's supported on edge endpoints
        let resp = self.ingest_http.post(path, &req).await?;

        let saved_query_id = resp
            .headers()
            .get("X-Axiom-History-Query-Id")
            .map(|s| s.to_str())
            .transpose()
            .map_err(|_e| Error::InvalidQueryId)?
            .map(ToString::to_string);

        let trace_id = resp
            .headers()
            .get("x-axiom-trace-id")
            .map(|s| s.to_str())
            .transpose()
            .map_err(|_e| Error::InvalidTraceId)?
            .map(ToString::to_string);
        let mut result = resp.json::<QueryResult>().await?;
        result.saved_query_id = saved_query_id;
        result.trace_id = trace_id;

        Ok(result)
    }

    /// Ingest events into the dataset identified by its id.
    /// Restrictions for field names (JSON object keys) can be reviewed here:
    /// <https://www.axiom.co/docs/usage/field-restrictions>.
    ///
    /// # Errors
    ///
    /// Returns an error if the events cannot be serialized or if the HTTP
    /// request or JSON deserializing fails.
    #[instrument(skip(self, events))]
    pub async fn ingest<N, I, E>(&self, dataset_name: N, events: I) -> Result<IngestStatus>
    where
        N: Into<String> + FmtDebug,
        I: IntoIterator<Item = E>,
        E: Serialize,
    {
        let json_lines: Result<Vec<Vec<u8>>> = events
            .into_iter()
            .map(|event| serde_json::to_vec(&event).map_err(Error::Serialize))
            .collect();
        let json_payload = json_lines?.join(&b"\n"[..]);
        let payload = spawn_blocking(move || {
            let mut gzip_payload = GzEncoder::new(Vec::new(), Compression::default());
            gzip_payload.write_all(&json_payload)?;
            gzip_payload.finish()
        })
        .await;
        #[cfg(feature = "tokio")]
        let payload = payload.map_err(Error::JoinError)?;
        let payload = payload.map_err(Error::Encoding)?;

        self.ingest_bytes(
            dataset_name,
            payload,
            ContentType::NdJson,
            ContentEncoding::Gzip,
        )
        .await
    }

    /// Ingest data into the dataset identified by its id.
    /// Restrictions for field names (JSON object keys) can be reviewed here:
    /// <https://www.axiom.co/docs/usage/field-restrictions>.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request or JSON deserializing fails.
    #[instrument(skip(self, payload))]
    pub async fn ingest_bytes<N, P>(
        &self,
        dataset_name: N,
        payload: P,
        content_type: ContentType,
        content_encoding: ContentEncoding,
    ) -> Result<IngestStatus>
    where
        N: Into<String> + FmtDebug,
        P: Into<Bytes>,
    {
        self.ingest_bytes_opt(
            dataset_name,
            payload,
            content_type,
            content_encoding,
            RequestOptions::default(),
        )
        .await
    }

    /// Like `ingest_bytes`, but takes a `RequestOptions`, which allows you to
    /// customize your request further.
    /// Note that any content-type and content-type headers in `RequestOptions`
    /// will be overwritten by the given arguments.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request or JSON deserializing fails.
    #[instrument(skip(self, payload))]
    pub async fn ingest_bytes_opt<N, P>(
        &self,
        dataset_name: N,
        payload: P,
        content_type: ContentType,
        content_encoding: ContentEncoding,
        request_options: RequestOptions,
    ) -> Result<IngestStatus>
    where
        N: Into<String> + FmtDebug,
        P: Into<Bytes>,
    {
        let mut headers = HeaderMap::new();

        // Add headers from request options
        for (key, value) in request_options.additional_headers {
            if let Some(key) = key {
                headers.insert(key, value);
            }
        }

        // Add content-type, content-encoding
        headers.insert(header::CONTENT_TYPE, content_type.into());
        headers.insert(header::CONTENT_ENCODING, content_encoding.into());

        let dataset_name = dataset_name.into();
        // Edge endpoints use /v1/ingest/{dataset}, legacy uses /v1/datasets/{dataset}/ingest
        let path = if self.uses_edge {
            format!("/v1/ingest/{dataset_name}")
        } else {
            format!("/v1/datasets/{dataset_name}/ingest")
        };

        self.ingest_http
            .post_bytes(path, payload, headers)
            .await?
            .json()
            .await
    }

    /// Ingest a stream of events into a dataset. Events will be ingested in
    /// chunks of 1000 items. If ingestion of a chunk fails, it will be retried
    /// with a backoff.
    /// Restrictions for field names (JSON object keys) can be reviewed here:
    /// <https://www.axiom.co/docs/usage/field-restrictions>.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request or JSON deserializing fails.
    #[instrument(skip(self, stream))]
    pub async fn ingest_stream<N, S, E>(&self, dataset_name: N, stream: S) -> Result<IngestStatus>
    where
        N: Into<String> + FmtDebug,
        S: Stream<Item = E> + Send + Sync + 'static,
        E: Serialize,
    {
        let dataset_name = dataset_name.into();
        let mut chunks = Box::pin(stream.chunks_timeout(1000, StdDuration::from_secs(1)));
        let mut ingest_status = IngestStatus::default();
        while let Some(events) = chunks.next().await {
            let new_ingest_status = self.ingest(dataset_name.clone(), events).await?;
            ingest_status = ingest_status + new_ingest_status;
        }
        Ok(ingest_status)
    }

    /// Like [`Client::ingest_stream`], but takes a stream that contains results.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request or JSON deserializing fails.
    #[instrument(skip(self, stream))]
    pub async fn try_ingest_stream<N, S, I, E>(
        &self,
        dataset_name: N,
        stream: S,
    ) -> Result<IngestStatus>
    where
        N: Into<String> + FmtDebug,
        S: Stream<Item = StdResult<I, E>> + Send + Sync + 'static,
        I: Serialize,
        E: std::error::Error + Send + Sync + 'static,
    {
        let dataset_name = dataset_name.into();
        let mut chunks = Box::pin(stream.chunks_timeout(1000, StdDuration::from_secs(1)));
        let mut ingest_status = IngestStatus::default();
        while let Some(events) = chunks.next().await {
            let events: StdResult<Vec<I>, E> = events.into_iter().collect();
            match events {
                Ok(events) => {
                    let new_ingest_status = self.ingest(dataset_name.clone(), events).await?;
                    ingest_status = ingest_status + new_ingest_status;
                }
                Err(e) => return Err(Error::IngestStreamError(Box::new(e))),
            }
        }
        Ok(ingest_status)
    }
}

/// This builder is used to create a new client.
pub struct Builder {
    env_fallback: bool,
    url: Option<String>,
    ingest_url: Option<String>,
    region: Option<String>,
    token: Option<String>,
    org_id: Option<String>,
}

impl Builder {
    /// Create a new builder.
    fn new() -> Self {
        Self {
            env_fallback: true,
            url: None,
            ingest_url: None,
            region: None,
            token: None,
            org_id: None,
        }
    }

    /// Don't fall back to environment variables.
    #[must_use]
    pub fn no_env(mut self) -> Self {
        self.env_fallback = false;
        self
    }

    /// Add a token to the client. If this is not set, the token will be read
    /// from the environment variable `AXIOM_TOKEN`.
    #[must_use]
    pub fn with_token<S: Into<String>>(mut self, token: S) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Add an URL to the client. This is only meant for testing purposes, you
    /// don't need to set it.
    #[must_use]
    pub fn with_url<S: Into<String>>(mut self, url: S) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Add an organization ID to the client. If this is not set, the
    /// organization ID will be read from the environment variable `AXIOM_ORG_ID`.
    #[must_use]
    pub fn with_org_id<S: Into<String>>(mut self, org_id: S) -> Self {
        self.org_id = Some(org_id.into());
        self
    }

    /// Set the Axiom regional edge domain for ingestion.
    ///
    /// Specify the domain name only (no scheme, no path).
    /// When set, data is sent to `https://{region}/v1/ingest/{dataset}`.
    ///
    /// If this is not set, the region will be read from the environment
    /// variable `AXIOM_REGION`.
    ///
    /// # Examples
    /// ```no_run
    /// use axiom_rs::Client;
    ///
    /// let client = Client::builder()
    ///     .with_token("my-token")
    ///     .with_region("eu-central-1.aws.edge.axiom.co")
    ///     .build();
    /// ```
    #[must_use]
    pub fn with_region<S: Into<String>>(mut self, region: S) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set an explicit ingest URL for the client.
    ///
    /// This takes precedence over `with_region`. Use this when you need
    /// full control over the ingest endpoint URL.
    ///
    /// If this is not set, the ingest URL will be read from the environment
    /// variable `AXIOM_INGEST_URL`.
    #[must_use]
    pub fn with_ingest_url<S: Into<String>>(mut self, ingest_url: S) -> Self {
        self.ingest_url = Some(ingest_url.into());
        self
    }

    /// Build the client.
    ///
    /// # Errors
    /// If the client can not be built
    pub fn build(self) -> Result<Client> {
        let env_fallback = self.env_fallback;

        let mut token = self.token.unwrap_or_default();
        if token.is_empty() && env_fallback {
            token = env::var("AXIOM_TOKEN").unwrap_or_default();
        }
        if token.is_empty() {
            return Err(Error::MissingToken);
        }

        // Resolve API URL
        let mut api_url = self.url.unwrap_or_default();
        if api_url.is_empty() && env_fallback {
            api_url = env::var("AXIOM_URL").unwrap_or_default();
        }
        if api_url.is_empty() {
            api_url = API_URL.to_string();
        }

        let mut org_id = self.org_id.unwrap_or_default();
        if org_id.is_empty() && env_fallback {
            org_id = env::var("AXIOM_ORG_ID").unwrap_or_default();
        }

        // On Cloud you need an Org ID for Personal Tokens.
        if api_url == API_URL && org_id.is_empty() && is_personal_token(&token) {
            return Err(Error::MissingOrgId);
        }

        // Resolve ingest URL and region
        // Priority: ingest_url > region > api_url (for backwards compatibility)
        let mut ingest_url = self.ingest_url.unwrap_or_default();
        if ingest_url.is_empty() && env_fallback {
            ingest_url = env::var("AXIOM_INGEST_URL").unwrap_or_default();
        }

        let mut region = self.region.unwrap_or_default();
        if region.is_empty() && env_fallback {
            region = env::var("AXIOM_REGION").unwrap_or_default();
        }

        // Determine ingest URL and whether we're using edge endpoints
        // Priority: ingest_url > region > default
        // Edge mode is determined by: region being set OR ingest_url looking like an edge URL
        let uses_edge = !region.is_empty()
            || ingest_url.contains(".edge.")
            || ingest_url.contains("/v1/ingest")
            || (ingest_url.is_empty() && api_url == API_URL);

        let ingest_url = if !ingest_url.is_empty() {
            // Explicit ingest URL takes precedence
            ingest_url
        } else if !region.is_empty() {
            // Region specified - build edge URL
            let region = region.trim_end_matches('/');
            format!("https://{region}")
        } else if api_url == API_URL {
            // Default cloud: use default edge endpoint for ingest
            DEFAULT_EDGE_URL.to_string()
        } else {
            // Custom API URL without region - use same URL for ingest (backwards compatible)
            api_url.clone()
        };

        let org_id_opt: Option<String> = if org_id.is_empty() {
            None
        } else {
            Some(org_id)
        };
        let api_http = http::Client::new(api_url.clone(), token.clone(), org_id_opt.clone())?;
        let ingest_http = http::Client::new(ingest_url.clone(), token, org_id_opt)?;

        Ok(Client {
            api_http,
            ingest_http,
            api_url,
            ingest_url,
            uses_edge,
        })
    }
}
