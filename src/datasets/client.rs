#[cfg(feature = "async-std")]
use async_std::task::spawn_blocking;
use bytes::Bytes;
use flate2::{write::GzEncoder, Compression};
use futures::Stream;
use reqwest::header;
use serde::Serialize;
use std::{
    convert::{TryFrom, TryInto},
    fmt::Debug as FmtDebug,
    io::Write,
    result::Result as StdResult,
    time::Duration as StdDuration,
};
#[cfg(feature = "tokio")]
use tokio::task::spawn_blocking;
use tokio_stream::StreamExt;
use tracing::instrument;

use crate::{
    datasets::model::*,
    error::{Error, Result},
    http::{self, HeaderMap},
};

/// Provides methods to work with Axiom datasets, including ingesting and
/// querying.
/// If you're looking for the query methods, those are at the
/// [top-level client](crate::Client).
#[derive(Debug, Clone)]
pub struct Client {
    http_client: http::Client,
}

impl Client {
    pub(crate) fn new(http_client: http::Client) -> Self {
        Self { http_client }
    }

    /// Create a dataset with the given name and description.
    #[instrument(skip(self))]
    pub async fn create<N, D>(&self, dataset_name: N, description: D) -> Result<Dataset>
    where
        N: Into<String> + FmtDebug,
        D: Into<String> + FmtDebug,
    {
        let req = DatasetCreateRequest {
            name: dataset_name.into(),
            description: description.into(),
        };
        self.http_client
            .post("/v1/datasets", &req)
            .await?
            .json()
            .await
    }

    /// Delete the dataset with the given ID.
    #[instrument(skip(self))]
    pub async fn delete<N>(&self, dataset_name: N) -> Result<()>
    where
        N: Into<String> + FmtDebug,
    {
        self.http_client
            .delete(format!("/v1/datasets/{}", dataset_name.into()))
            .await
    }

    /// Get a dataset by its id.
    #[instrument(skip(self))]
    pub async fn get<N>(&self, dataset_name: N) -> Result<Dataset>
    where
        N: Into<String> + FmtDebug,
    {
        self.http_client
            .get(format!("/v1/datasets/{}", dataset_name.into()))
            .await?
            .json()
            .await
    }

    /// Retrieve the information of the dataset identified by its id.
    #[instrument(skip(self))]
    pub async fn info<N>(&self, dataset_name: N) -> Result<Info>
    where
        N: Into<String> + FmtDebug,
    {
        self.http_client
            .get(format!("/v1/datasets/{}/info", dataset_name.into()))
            .await?
            .json()
            .await
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

        self.ingest_raw(
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
    pub async fn ingest_raw<N, P>(
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
            ingest_status = ingest_status + new_ingest_status
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
                    ingest_status = ingest_status + new_ingest_status
                }
                Err(e) => return Err(Error::IngestStreamError(Box::new(e))),
            }
        }
        Ok(ingest_status)
    }

    /// List all available datasets.
    #[instrument(skip(self))]
    pub async fn list(&self) -> Result<Vec<Dataset>> {
        self.http_client.get("/v1/datasets").await?.json().await
    }

    /// Trim the dataset identified by its id to a given length.
    /// The max duration given will mark the oldest timestamp an event can have.
    /// Older ones will be deleted from the dataset.
    /// The duration can either be a [`std::time::Duration`] or a
    /// [`chrono::Duration`].
    #[instrument(skip(self))]
    pub async fn trim<N, D>(&self, dataset_name: N, duration: D) -> Result<TrimResult>
    where
        N: Into<String> + FmtDebug,
        D: TryInto<Duration, Error = Error> + FmtDebug,
    {
        let duration = duration.try_into()?;
        let req = TrimRequest::new(duration.into());
        self.http_client
            .post(format!("/v1/datasets/{}/trim", dataset_name.into()), &req)
            .await?
            .json()
            .await
    }

    /// Update a dataset.
    #[instrument(skip(self))]
    pub async fn update<N>(&self, dataset_name: N, req: DatasetUpdateRequest) -> Result<Dataset>
    where
        N: Into<String> + FmtDebug,
    {
        self.http_client
            .put(format!("/v1/datasets/{}", dataset_name.into()), &req)
            .await?
            .json()
            .await
    }
}

pub struct Duration {
    inner: chrono::Duration,
}

impl From<Duration> for chrono::Duration {
    fn from(duration: Duration) -> Self {
        duration.inner
    }
}

impl TryFrom<chrono::Duration> for Duration {
    type Error = Error;

    fn try_from(duration: chrono::Duration) -> StdResult<Self, Self::Error> {
        Ok(Duration { inner: duration })
    }
}

impl TryFrom<StdDuration> for Duration {
    type Error = Error;

    fn try_from(duration: StdDuration) -> StdResult<Self, Self::Error> {
        Ok(Duration {
            inner: chrono::Duration::from_std(duration).map_err(|_| Error::DurationOutOfRange)?,
        })
    }
}
