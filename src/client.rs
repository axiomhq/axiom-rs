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

/// Determines how ingest and query URLs are constructed.
/// Each variant contains the base URL for the endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PathStyle {
    /// Legacy path format:
    /// - Ingest: `/v1/datasets/{dataset}/ingest`
    /// - Query: `/v1/datasets/_apl`
    ///
    /// Used when no edge config is set.
    Legacy(String),
    /// Edge path format:
    /// - Ingest: `/v1/ingest/{dataset}`
    /// - Query: `/v1/query/_apl`
    ///
    /// Used when `edge` domain is set, or `edge_url` is set without a path.
    Edge(String),
    /// URL has a custom path - use as-is without appending any path.
    ///
    /// Used when `edge_url` contains a path component.
    AsIs(String),
}

impl PathStyle {
    /// Returns the base URL for this path style.
    pub(crate) fn url(&self) -> &str {
        match self {
            PathStyle::Legacy(url) | PathStyle::Edge(url) | PathStyle::AsIs(url) => url,
        }
    }

    /// Returns the ingest path for the given dataset.
    pub(crate) fn ingest_path(&self, dataset_name: &str) -> String {
        match self {
            PathStyle::Legacy(_) => format!("/v1/datasets/{dataset_name}/ingest"),
            PathStyle::Edge(_) => format!("/v1/ingest/{dataset_name}"),
            PathStyle::AsIs(_) => String::new(),
        }
    }

    /// Returns the query path with the given query parameters.
    pub(crate) fn query_path(&self, query_params: &str) -> String {
        match self {
            PathStyle::Legacy(_) => format!("/v1/datasets/_apl?{query_params}"),
            PathStyle::Edge(_) => format!("/v1/query/_apl?{query_params}"),
            PathStyle::AsIs(_) => format!("?{query_params}"),
        }
    }

    /// Returns true if this is an edge path style.
    pub(crate) fn is_edge(&self) -> bool {
        matches!(self, PathStyle::Edge(_) | PathStyle::AsIs(_))
    }
}

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
///         .with_edge("eu-central-1.aws.edge.axiom.co")
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
    edge_http: http::Client,
    /// The API URL.
    api_url: String,
    /// How ingest and query paths should be constructed (includes edge URL).
    path_style: PathStyle,
    /// Whether a personal token is being used.
    is_personal_token: bool,
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
    pub fn edge_url(&self) -> &str {
        self.path_style.url()
    }

    /// Returns true if the client is configured to use an edge endpoint.
    #[must_use]
    pub fn uses_edge(&self) -> bool {
        self.path_style.is_edge()
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
        let path = self.path_style.query_path(&query_params);
        let resp = self.edge_http.post(path, &req).await?;

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
        // Edge ingest does not support personal tokens
        if self.uses_edge() && self.is_personal_token {
            return Err(Error::PersonalTokenNotSupportedForEdge);
        }

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
        let path = self.path_style.ingest_path(&dataset_name);

        self.edge_http
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
    edge_url: Option<String>,
    token: Option<String>,
    org_id: Option<String>,
}

impl Builder {
    /// Create a new builder.
    fn new() -> Self {
        Self {
            env_fallback: true,
            url: None,
            edge_url: None,
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

    /// Set the Axiom regional edge domain for ingestion and query.
    ///
    /// Specify the domain name only (no scheme, no path). The `https://` scheme
    /// will be automatically prepended.
    /// When set, ingest requests are sent to `https://{edge}/v1/ingest/{dataset}`
    /// and query requests are sent to `https://{edge}/v1/query/_apl`.
    ///
    /// If this is not set, the edge domain will be read from the environment
    /// variable `AXIOM_EDGE`.
    ///
    /// Note: Edge endpoints require API tokens (`xaat-`) for ingestion.
    ///
    /// # Examples
    /// ```no_run
    /// use axiom_rs::Client;
    ///
    /// let client = Client::builder()
    ///     .with_token("xaat-my-api-token")
    ///     .with_edge("eu-central-1.aws.edge.axiom.co")
    ///     .build();
    /// ```
    #[must_use]
    pub fn with_edge<S: Into<String>>(mut self, edge: S) -> Self {
        let edge = edge.into().trim_end_matches('/').to_string();
        self.edge_url = Some(format!("https://{edge}"));
        self
    }

    /// Set an explicit edge URL for the client.
    ///
    /// Use this when you need full control over the edge endpoint URL,
    /// for example when using a custom load balancer.
    ///
    /// If this is not set, the edge URL will be read from the environment
    /// variable `AXIOM_EDGE_URL`.
    ///
    /// Note: Edge endpoints require API tokens (`xaat-`) for ingestion.
    #[must_use]
    pub fn with_edge_url<S: Into<String>>(mut self, edge_url: S) -> Self {
        self.edge_url = Some(edge_url.into());
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

        // Resolve edge URL
        // Priority: edge_url (from builder or AXIOM_EDGE_URL) > AXIOM_EDGE > api_url
        let mut edge_url = self.edge_url.unwrap_or_default();
        if edge_url.is_empty() && env_fallback {
            edge_url = env::var("AXIOM_EDGE_URL").unwrap_or_default();
        }
        if edge_url.is_empty() && env_fallback {
            // Fall back to AXIOM_EDGE (domain only) and prepend https://
            let edge = env::var("AXIOM_EDGE").unwrap_or_default();
            if !edge.is_empty() {
                let edge = edge.trim_end_matches('/');
                edge_url = format!("https://{edge}");
            }
        }

        // Determine path style (which includes the URL)
        // - edge_url with path: use AsIs style (URL used as-is)
        // - edge_url without path: use Edge style
        // - no edge config: use Legacy style with api_url
        let path_style = if edge_url.is_empty() {
            // No edge config - use API URL with legacy path format
            PathStyle::Legacy(api_url.clone())
        } else {
            // Check if URL has a custom path
            let has_custom_path = if let Ok(parsed) = url::Url::parse(&edge_url) {
                let path = parsed.path();
                !path.is_empty() && path != "/"
            } else {
                false
            };

            if has_custom_path {
                // URL has custom path - use as-is
                PathStyle::AsIs(edge_url)
            } else {
                // URL without path - use edge path format
                PathStyle::Edge(edge_url)
            }
        };

        let is_personal_token = is_personal_token(&token);
        let org_id_opt: Option<String> = if org_id.is_empty() {
            None
        } else {
            Some(org_id)
        };
        let api_http = http::Client::new(api_url.clone(), token.clone(), org_id_opt.clone())?;
        let edge_http = http::Client::new(path_style.url(), token, org_id_opt)?;

        Ok(Client {
            api_http,
            edge_http,
            api_url,
            path_style,
            is_personal_token,
        })
    }
}
