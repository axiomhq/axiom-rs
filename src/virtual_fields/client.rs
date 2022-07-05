use crate::{error::Result, http, virtual_fields::model::*};

/// Provides methods to work with virtual fields.
pub struct Client {
    http_client: http::Client,
}

impl Client {
    pub(crate) fn new(http_client: http::Client) -> Self {
        Self { http_client }
    }

    /// Get all available virtual fields.
    pub async fn list(&self, opts: ListOptions) -> Result<Vec<VirtualField>> {
        let query_string = serde_qs::to_string(&opts)?;

        self.http_client
            .get(format!("/vfields?{}", query_string))
            .await?
            .json()
            .await
    }

    /// Get a virtual field by ID.
    pub async fn get<S>(&self, id: S) -> Result<VirtualField>
    where
        S: Into<String>,
    {
        self.http_client
            .get(format!("/vfields/{}", id.into()))
            .await?
            .json()
            .await
    }

    /// Create a new virtual field.
    pub async fn create(
        &self,
        virtual_field: VirtualFieldCreateUpdateRequest,
    ) -> Result<VirtualField> {
        self.http_client
            .post("/vfields", &virtual_field)
            .await?
            .json()
            .await
    }

    /// Update a virtual field.
    pub async fn update<S>(
        &self,
        id: S,
        virtual_field: VirtualFieldCreateUpdateRequest,
    ) -> Result<VirtualField>
    where
        S: Into<String>,
    {
        self.http_client
            .put(format!("/vfields/{}", id.into()), &virtual_field)
            .await?
            .json()
            .await
    }

    /// Delete a virtual field.
    pub async fn delete<S>(&self, id: S) -> Result<()>
    where
        S: Into<String>,
    {
        self.http_client
            .delete(format!("/vfields/{}", id.into()))
            .await?;
        Ok(())
    }
}
