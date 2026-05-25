use std::fmt::Debug as FmtDebug;
use tracing::instrument;

use crate::{
    dashboards::model::{Dashboard, DashboardDocument, DashboardWriteResponse, UpsertRequest},
    error::{Axiom, Error, Result},
    http,
};

/// Provides methods to work with Axiom dashboards (`v2` API).
///
/// Obtain an instance via [`crate::Client::dashboards`]. All methods hit the
/// control-plane URL configured on the parent client.
#[derive(Debug, Clone)]
pub struct Client<'client> {
    http_client: &'client http::Client,
}

impl<'client> Client<'client> {
    pub(crate) fn new(http_client: &'client http::Client) -> Self {
        Self { http_client }
    }

    /// List the dashboards visible to the token, capped at the server's
    /// maximum page size (1000).
    ///
    /// When authenticated with an API token, the server only returns
    /// dashboards shared org-wide or with a group; private dashboards are
    /// invisible to API tokens.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the response cannot be
    /// deserialised.
    #[instrument(skip(self))]
    pub async fn list(&self) -> Result<Vec<Dashboard>> {
        self.http_client
            .get("/v2/dashboards?limit=1000")
            .await?
            .json()
            .await
    }

    /// Fetch a single dashboard by its `uid`.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the response cannot be
    /// deserialised.
    #[instrument(skip(self))]
    pub async fn get<U>(&self, uid: U) -> Result<Dashboard>
    where
        U: Into<String> + FmtDebug,
    {
        let uid = uid.into();
        self.http_client
            .get(format!("/v2/dashboards/uid/{uid}"))
            .await?
            .json()
            .await
    }

    /// Create a new dashboard.
    ///
    /// `uid` is optional; when omitted the server assigns one derived from
    /// the dashboard's name.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails, the response cannot be
    /// deserialised, or the server reports an error.
    #[instrument(skip(self, doc))]
    pub async fn create(
        &self,
        doc: &DashboardDocument,
        uid: Option<&str>,
        message: Option<&str>,
    ) -> Result<DashboardWriteResponse> {
        let body = UpsertRequest {
            dashboard: doc,
            version: None,
            overwrite: false,
            uid,
            message,
        };
        let resp = self.http_client.post("/v2/dashboards", body).await?;
        decode_write_response(resp, ::http::Method::POST, "/v2/dashboards".to_string()).await
    }

    /// Replace an existing dashboard, addressing it by `uid`.
    ///
    /// `expected_version` is the version you loaded; pass it unless
    /// `overwrite` is `true`, in which case the server skips the version
    /// check. A version mismatch surfaces as
    /// [`Error::DashboardVersionConflict`] populated with the server's
    /// current version.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails, the response cannot be
    /// deserialised, or the server reports an error.
    #[instrument(skip(self, doc))]
    pub async fn put<U>(
        &self,
        uid: U,
        doc: &DashboardDocument,
        expected_version: Option<i64>,
        overwrite: bool,
        message: Option<&str>,
    ) -> Result<DashboardWriteResponse>
    where
        U: Into<String> + FmtDebug,
    {
        let uid = uid.into();
        let path = format!("/v2/dashboards/uid/{uid}");
        let body = UpsertRequest {
            dashboard: doc,
            version: if overwrite { None } else { expected_version },
            overwrite,
            uid: Some(&uid),
            message,
        };
        let resp = self.http_client.put(&path, body).await?;
        decode_write_response(resp, ::http::Method::PUT, path).await
    }

    /// Delete a dashboard by `uid`.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the server reports an
    /// error.
    #[instrument(skip(self))]
    pub async fn delete<U>(&self, uid: U) -> Result<()>
    where
        U: Into<String> + FmtDebug,
    {
        let uid = uid.into();
        self.http_client
            .delete(format!("/v2/dashboards/uid/{uid}"))
            .await
    }
}

/// Decode the write response, mapping `412` to a typed
/// [`Error::DashboardVersionConflict`] using the dashboard error envelope
/// (which carries `currentVersion`).
async fn decode_write_response(
    resp: http::Response,
    method: ::http::Method,
    path: String,
) -> Result<DashboardWriteResponse> {
    let inner: reqwest::Response = resp.into();
    let status = inner.status();
    let trace_id = inner
        .headers()
        .get("x-axiom-trace-id")
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);
    let text = inner.text().await.map_err(Error::Http)?;

    if status.is_success() {
        return serde_json::from_str::<DashboardWriteResponse>(&text).map_err(Error::from);
    }

    let err = serde_json::from_str::<DashboardErrorBody>(&text).ok();
    if status.as_u16() == 412 {
        if let Some(current) = err.as_ref().and_then(|e| e.current_version) {
            return Err(Error::DashboardVersionConflict {
                uid: err.as_ref().and_then(|e| e.uid.clone()).unwrap_or_default(),
                current,
            });
        }
    }
    Err(Error::Axiom(Axiom::new(
        status.as_u16(),
        method,
        path,
        err.and_then(|e| {
            if e.message.is_empty() {
                None
            } else {
                Some(e.message)
            }
        }),
        trace_id,
    )))
}

#[derive(Debug, serde::Deserialize)]
struct DashboardErrorBody {
    #[serde(default)]
    message: String,
    #[serde(rename = "currentVersion", default)]
    current_version: Option<i64>,
    #[serde(default)]
    uid: Option<String>,
}
