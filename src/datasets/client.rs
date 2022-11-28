use std::{
    convert::{TryFrom, TryInto},
    fmt::Debug as FmtDebug,
    result::Result as StdResult,
    time::Duration as StdDuration,
};
use tracing::instrument;

use crate::{
    datasets::model::*,
    error::{Error, Result},
    http,
};

/// Provides methods to work with Axiom datasets, including ingesting and
/// querying.
/// If you're looking for the ingest and query methods, those are at the
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
