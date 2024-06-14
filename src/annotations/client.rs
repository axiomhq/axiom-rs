use std::fmt;

use crate::{
    annotations::model::{Annotation, AnnotationRequest},
    error::Result,
    http,
};
use tracing::instrument;

use super::ListRequest;

/// Provides methods to work with Axiom datasets.
#[derive(Debug, Clone)]
pub struct Client<'client> {
    http_client: &'client http::Client,
}

impl<'client> Client<'client> {
    pub(crate) fn new(http_client: &'client http::Client) -> Self {
        Self { http_client }
    }

    /// creates an annotaion
    ///
    /// # Errors
    /// If the API call fails
    #[instrument(skip(self))]
    pub async fn create(&self, req: AnnotationRequest) -> Result<Annotation> {
        self.http_client
            .post("/v2/annotations", req)
            .await?
            .json()
            .await
    }

    /// gets an annotaion
    ///
    /// # Errors
    /// If the API call fails
    #[instrument(skip(self))]
    pub async fn get(&self, id: impl fmt::Display + fmt::Debug) -> Result<Annotation> {
        self.http_client
            .get(format!("/v2/annotations/{id}"))
            .await?
            .json()
            .await
    }

    /// lists annotaions
    ///
    /// # Errors
    /// If the API call fails
    #[instrument(skip(self))]
    pub async fn list(&self, req: ListRequest) -> Result<Vec<Annotation>> {
        let query_params = serde_qs::to_string(&req)?;
        self.http_client
            .get(format!("/v2/annotations?{query_params}"))
            .await?
            .json()
            .await
    }
}
