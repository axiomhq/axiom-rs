use std::fmt::Debug as FmtDebug;
use serde::Serialize;
use tracing::instrument;

use crate::{
    dashboards::model::{
        CreateOptions, Dashboard, DashboardDocument, DashboardWriteResponse, UpsertOptions,
        UpsertRequest,
    },
    error::{Axiom, Error, Result},
    http,
};

/// Server's maximum page size for `GET /v2/dashboards`.
const LIST_LIMIT: u32 = 1000;

/// Query string for the dashboards list endpoint. Round-tripped through
/// `serde_qs` for consistency with the rest of the SDK.
#[derive(Serialize)]
struct ListParams {
    limit: u32,
}

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
        let qs = serde_qs::to_string(&ListParams { limit: LIST_LIMIT })?;
        self.http_client
            .get(format!("/v2/dashboards?{qs}"))
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
    pub async fn get(&self, uid: impl Into<String> + FmtDebug) -> Result<Dashboard> {
        let uid = uid.into();
        self.http_client
            .get(format!("/v2/dashboards/uid/{uid}"))
            .await?
            .json()
            .await
    }

    /// Create a new dashboard.
    ///
    /// Pass [`CreateOptions::default()`] (or `None`) for the common case;
    /// supply a `uid` to request a specific one (otherwise the server
    /// assigns one derived from the dashboard's name) and/or a `message`
    /// for the audit log.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails, the response cannot be
    /// deserialised, or the server reports an error. See
    /// [`Error::DashboardVersionConflict`] for the optimistic-concurrency
    /// failure mode (`put` only).
    #[instrument(skip(self, doc, opts))]
    pub async fn create<O>(
        &self,
        doc: &DashboardDocument,
        opts: O,
    ) -> Result<DashboardWriteResponse>
    where
        O: Into<Option<CreateOptions>>,
    {
        let opts = opts.into().unwrap_or_default();
        let body = UpsertRequest {
            dashboard: doc,
            version: None,
            overwrite: false,
            uid: opts.uid.as_deref(),
            message: opts.message.as_deref(),
        };
        let resp = self.http_client.post("/v2/dashboards", body).await?;
        // `create` cannot raise an optimistic-version conflict (there's no
        // prior version), so a 412 here would be just as unexpected as on
        // any other write; passing the (optional) caller-supplied uid is
        // harmless and keeps the helper uniform.
        decode_write_response(resp, opts.uid.as_deref().unwrap_or("")).await
    }

    /// Replace an existing dashboard, addressing it by `uid`.
    ///
    /// Pass [`UpsertOptions::default()`] (or `None`) for a write with the
    /// server's default version-check policy. Set
    /// [`UpsertOptions::expected_version`] to the version you loaded for
    /// strict optimistic concurrency, or [`UpsertOptions::overwrite`] to
    /// skip the check. A version mismatch surfaces as
    /// [`Error::DashboardVersionConflict`] populated with the server's
    /// current version.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails, the response cannot be
    /// deserialised, the server reports an error, or the optimistic version
    /// check fails (see [`Error::DashboardVersionConflict`]).
    #[instrument(skip(self, doc, opts))]
    pub async fn put<O>(
        &self,
        uid: impl Into<String> + FmtDebug,
        doc: &DashboardDocument,
        opts: O,
    ) -> Result<DashboardWriteResponse>
    where
        O: Into<Option<UpsertOptions>>,
    {
        let uid = uid.into();
        let opts = opts.into().unwrap_or_default();
        let path = format!("/v2/dashboards/uid/{uid}");
        let body = UpsertRequest {
            dashboard: doc,
            version: if opts.overwrite {
                None
            } else {
                opts.expected_version
            },
            overwrite: opts.overwrite,
            uid: Some(&uid),
            message: opts.message.as_deref(),
        };
        let resp = self.http_client.put(&path, body).await?;
        decode_write_response(resp, &uid).await
    }

    /// Delete a dashboard by `uid`.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the server reports an
    /// error.
    #[instrument(skip(self))]
    pub async fn delete(&self, uid: impl Into<String> + FmtDebug) -> Result<()> {
        let uid = uid.into();
        self.http_client
            .delete(format!("/v2/dashboards/uid/{uid}"))
            .await
    }
}

/// Decode the write response.
///
/// Happy path: deserialise the [`DashboardWriteResponse`] envelope via the
/// shared `check_error` path so rate-limit / query-limit / ingest-limit
/// mapping all keep working.
///
/// 412 conflict: read the raw body so we can extract `currentVersion` and
/// surface a typed [`Error::DashboardVersionConflict`]. If the body lacks
/// `currentVersion` (or doesn't decode), fall back to the generic
/// [`Error::Axiom`] envelope rather than panicking on the decode.
///
/// `caller_uid` is the uid the SDK sent in the request, used as the
/// fallback when the server's error body omits its own `uid` echo.
async fn decode_write_response(
    resp: http::Response,
    caller_uid: &str,
) -> Result<DashboardWriteResponse> {
    if resp.status() != ::http::StatusCode::PRECONDITION_FAILED {
        return resp.json::<DashboardWriteResponse>().await;
    }

    // Capture metadata we'll need to build either error variant before
    // consuming the body.
    let status = resp.status().as_u16();
    let method = resp.method().clone();
    let path = resp.path().to_string();
    let trace_id = resp
        .headers()
        .get("x-axiom-trace-id")
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);
    let body = resp.body_text_unchecked().await?;
    let parsed = serde_json::from_str::<DashboardErrorBody>(&body).ok();

    if let Some(err) = parsed.as_ref() {
        if let Some(current) = err.current_version {
            let uid = err
                .uid
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| caller_uid.to_string());
            return Err(Error::DashboardVersionConflict { uid, current });
        }
    }

    Err(Error::Axiom(Axiom::new(
        status,
        method,
        path,
        parsed.and_then(|e| {
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
