use crate::{error::Result, http, users::model::User};
use tracing::instrument;

/// Provides methods to work with Axiom datasets.
#[derive(Debug, Clone)]
pub struct Client<'client> {
    http_client: &'client http::Client,
}

impl<'client> Client<'client> {
    pub(crate) fn new(http_client: &'client http::Client) -> Self {
        Self { http_client }
    }

    /// Retrieve the authenticated user.
    #[instrument(skip(self))]
    pub async fn current(&self) -> Result<User> {
        self.http_client.get("/v1/user").await?.json().await
    }
}
