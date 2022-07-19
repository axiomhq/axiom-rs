use crate::{error::Result, http, users::model::*};

/// Provides methods to work with Axiom datasets.
pub struct Client {
    http_client: http::Client,
}

impl Client {
    pub(crate) fn new(http_client: http::Client) -> Self {
        Self { http_client }
    }

    /// Retrieve the authenticated user.
    pub async fn current(&self) -> Result<AuthenticatedUser> {
        self.http_client.get("/user").await?.json().await
    }
}
