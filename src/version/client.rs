use crate::{error::Result, http, version::model::*};

/// Provides methods to work with the Axiom version.
pub(crate) struct Client {
    http_client: http::Client,
}

impl Client {
    pub(crate) fn new(http_client: http::Client) -> Self {
        Self { http_client }
    }

    /// Returns the version of the Axiom deployment.
    pub(crate) async fn get(&self) -> Result<String> {
        let version: Version = self.http_client.get("/version").await?.json().await?;
        Ok(version.current_version)
    }
}
