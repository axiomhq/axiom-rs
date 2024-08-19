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
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    http_client: http::Client,
    url: String,
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
    pub fn datasets(&self) -> datasets::Client {
        datasets::Client::new(&self.http_client)
    }

    /// Users API
    #[must_use]
    pub fn users(&self) -> users::Client {
        users::Client::new(&self.http_client)
    }

    /// Annotations API
    #[must_use]
    pub fn annotations(&self) -> annotations::Client {
        annotations::Client::new(&self.http_client)
    }

    /// Get the API url
    #[doc(hidden)]
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get client version.
    #[must_use]
    pub fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    /// Executes the given query specified using the Axiom Processing Language (APL).
    /// To learn more about APL, see the APL documentation at https://www.axiom.co/docs/apl/introduction.
    #[instrument(skip(self, opts))]
    pub async fn query<S, O>(&self, apl: S, opts: O) -> Result<QueryResult>
    where
        S: Into<String> + FmtDebug,
        O: Into<Option<QueryOptions>>,
    {
        let (req, query_params) = match opts.into() {
            Some(opts) => {
                let req = Query {
                    apl: apl.into(),
                    start_time: opts.start_time,
                    end_time: opts.end_time,
                    cursor: opts.cursor,
                    include_cursor: opts.include_cursor,
                };

                let query_params = QueryParams {
                    no_cache: opts.no_cache,
                    save: opts.save,
                    format: opts.format,
                };

                (req, query_params)
            }
            None => (
                Query {
                    apl: apl.into(),
                    ..Default::default()
                },
                QueryParams::default(),
            ),
        };

        let query_params = serde_qs::to_string(&query_params)?;
        let path = format!("/v1/datasets/_apl?{query_params}");
        let res = self.http_client.post(path, &req).await?;

        let saved_query_id = res
            .headers()
            .get("X-Axiom-History-Query-Id")
            .map(|s| s.to_str())
            .transpose()
            .map_err(|_e| Error::InvalidQueryId)?
            .map(std::string::ToString::to_string);

        let mut result = res.json::<QueryResult>().await?;
        result.saved_query_id = saved_query_id;

        Ok(result)
    }

    /// Ingest events into the dataset identified by its id.
    /// Restrictions for field names (JSON object keys) can be reviewed here:
    /// <https://www.axiom.co/docs/usage/field-restrictions>.
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
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, content_type.into());
        headers.insert(header::CONTENT_ENCODING, content_encoding.into());

        self.http_client
            .post_bytes(
                format!("/v1/datasets/{}/ingest", dataset_name.into()),
                payload,
                headers,
            )
            .await?
            .json()
            .await
    }

    /// Ingest a stream of events into a dataset. Events will be ingested in
    /// chunks of 1000 items. If ingestion of a chunk fails, it will be retried
    /// with a backoff.
    /// Restrictions for field names (JSON object keys) can be reviewed here:
    /// <https://www.axiom.co/docs/usage/field-restrictions>.
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
    #[doc(hidden)]
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

        let mut url = self.url.unwrap_or_default();
        if url.is_empty() && env_fallback {
            url = env::var("AXIOM_URL").unwrap_or_default();
        }
        if url.is_empty() {
            url = API_URL.to_string();
        }

        let mut org_id = self.org_id.unwrap_or_default();
        if org_id.is_empty() && env_fallback {
            org_id = env::var("AXIOM_ORG_ID").unwrap_or_default();
        };

        // On Cloud you need an Org ID for Personal Tokens.
        if url == API_URL && org_id.is_empty() && is_personal_token(&token) {
            return Err(Error::MissingOrgId);
        }

        let http_client = http::Client::new(url.clone(), token, org_id)?;

        Ok(Client {
            http_client: http_client.clone(),
            url,
        })
    }
}
