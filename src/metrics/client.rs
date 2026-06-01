use std::{collections::BTreeMap, fmt::Debug as FmtDebug};

use std::fmt::Write;

use ::http::header::{HeaderMap, HeaderValue, ACCEPT};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tracing::instrument;

use crate::{
    error::{Error, Result},
    http,
    metrics::model::{MetricInfo, MetricsQueryResponse},
};

/// `?start=...&end=...` query string used by every metrics-info endpoint.
/// Serialised through `serde_qs` so we get consistent encoding without
/// hand-rolling a percent-encoder for the timestamps.
#[derive(Serialize)]
struct TimeRange {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

impl TimeRange {
    fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }

    /// Render as `start=...&end=...` (no leading `?`).
    fn to_query(&self) -> Result<String> {
        serde_qs::to_string(self).map_err(Error::from)
    }
}

/// Accept header for the metrics-info endpoint.
const ACCEPT_METRICS_INFO_V2: &str = "application/vnd.metrics-info.v2+json";
/// Accept header for the `_mpl` query endpoint.
const ACCEPT_METRICS_V2: &str = "application/json+metrics.v2";
/// Accept header for the self-describing MPL spec.
const ACCEPT_MARKDOWN: &str = "text/markdown";

/// Provides methods to work with Axiom metrics: metric discovery, tag
/// listing, and MPL queries against the edge endpoint.
///
/// Obtain an instance via [`crate::Client::metrics`].
#[derive(Debug, Clone)]
pub struct Client<'client> {
    http_client: &'client http::Client,
}

impl<'client> Client<'client> {
    pub(crate) fn new(http_client: &'client http::Client) -> Self {
        Self { http_client }
    }

    /// List the metrics observed in `dataset` between `start` and `end`.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the response cannot be
    /// deserialised.
    #[instrument(skip(self))]
    pub async fn list(
        &self,
        dataset: impl Into<String> + FmtDebug,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<BTreeMap<String, MetricInfo>> {
        let dataset = dataset.into();
        let qs = TimeRange::new(start, end).to_query()?;
        let path = format!("/v1/query/metrics/info/datasets/{dataset}/metrics?{qs}");
        self.http_client
            .get_with_headers(path, accept_header(ACCEPT_METRICS_INFO_V2))
            .await?
            .json()
            .await
    }

    /// List the tag names observed for `(dataset, metric)` between `start`
    /// and `end`.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the response cannot be
    /// deserialised.
    #[instrument(skip(self))]
    pub async fn tags(
        &self,
        dataset: impl Into<String> + FmtDebug,
        metric: impl Into<String> + FmtDebug,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<String>> {
        let dataset = dataset.into();
        let metric = encode_segment(&metric.into());
        let qs = TimeRange::new(start, end).to_query()?;
        let path = format!("/v1/query/metrics/info/datasets/{dataset}/metrics/{metric}/tags?{qs}");
        self.http_client.get(path).await?.json().await
    }

    /// List the observed values for `tag` of `(dataset, metric)` between
    /// `start` and `end`.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the response cannot be
    /// deserialised.
    #[instrument(skip(self))]
    pub async fn tag_values(
        &self,
        dataset: impl Into<String> + FmtDebug,
        metric: impl Into<String> + FmtDebug,
        tag: impl Into<String> + FmtDebug,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<String>> {
        let dataset = dataset.into();
        let metric = encode_segment(&metric.into());
        let tag = encode_segment(&tag.into());
        let qs = TimeRange::new(start, end).to_query()?;
        let path = format!(
            "/v1/query/metrics/info/datasets/{dataset}/metrics/{metric}/tags/{tag}/values?{qs}"
        );
        self.http_client.get(path).await?.json().await
    }

    /// List the tag names observed across **all metrics** of `dataset`
    /// between `start` and `end`.
    ///
    /// The per-metric pair lives on [`Client::tags`] / [`Client::tag_values`];
    /// this dataset-level variant is the right starting point when you
    /// don't yet know which metrics carry the tag you care about.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the response cannot be
    /// deserialised.
    #[instrument(skip(self))]
    pub async fn dataset_tags(
        &self,
        dataset: impl Into<String> + FmtDebug,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<String>> {
        let dataset = dataset.into();
        let qs = TimeRange::new(start, end).to_query()?;
        let path = format!("/v1/query/metrics/info/datasets/{dataset}/tags?{qs}");
        self.http_client.get(path).await?.json().await
    }

    /// List the observed values for a dataset-level `tag` across all
    /// metrics of `dataset` between `start` and `end`.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the response cannot be
    /// deserialised.
    #[instrument(skip(self))]
    pub async fn dataset_tag_values(
        &self,
        dataset: impl Into<String> + FmtDebug,
        tag: impl Into<String> + FmtDebug,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<String>> {
        let dataset = dataset.into();
        let tag = encode_segment(&tag.into());
        let qs = TimeRange::new(start, end).to_query()?;
        let path = format!("/v1/query/metrics/info/datasets/{dataset}/tags/{tag}/values?{qs}");
        self.http_client.get(path).await?.json().await
    }

    /// Find metrics in `dataset` that carry `value` on any tag, between
    /// `start` and `end`. Searches tag **values**, not metric names — use
    /// this when you know a specific entity (service, host, device) and
    /// want to find which metrics report it.
    ///
    /// Returns a map of **metric name → the tag name(s) that carried the
    /// searched value** on that metric, e.g. `{"http.server.duration":
    /// ["service.name"]}`. The tags are useful for building a precise
    /// follow-up filter (`where <tag> == <value>`) rather than guessing
    /// which spelling holds the value.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the response cannot be
    /// deserialised.
    #[instrument(skip(self))]
    pub async fn find_metrics(
        &self,
        dataset: impl Into<String> + FmtDebug,
        value: impl Into<String> + FmtDebug,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<BTreeMap<String, Vec<String>>> {
        let dataset = dataset.into();
        let qs = TimeRange::new(start, end).to_query()?;
        let path = format!("/v1/query/metrics/info/datasets/{dataset}/metrics?{qs}");
        let body = FindMetricsRequest {
            value: value.into(),
        };
        self.http_client.post(path, body).await?.json().await
    }

    /// Fetch the self-describing MPL spec as markdown.
    ///
    /// The MPL operator set evolves; this endpoint is the source of truth
    /// for syntax, operator names, and parameter literal formats. Useful
    /// for grounding LLM callers in the current surface.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the server reports an
    /// error.
    #[instrument(skip(self))]
    pub async fn spec(&self) -> Result<String> {
        self.http_client
            .options_with_headers("/v1/query/_mpl", accept_header(ACCEPT_MARKDOWN))
            .await?
            .text()
            .await
    }

    /// Run an MPL query against the edge endpoint.
    ///
    /// The request field name is `apl` for historical reasons; the payload
    /// is MPL, not APL.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails, the response cannot be
    /// deserialised, or a trace-id response header contains invalid bytes.
    #[instrument(skip(self, opts))]
    pub async fn query<S, O>(
        &self,
        mpl: &S,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        opts: O,
    ) -> Result<MetricsQueryResponse>
    where
        S: ToString + FmtDebug + ?Sized,
        O: Into<Option<MplQueryOptions>>,
    {
        let opts = opts.into().unwrap_or_default();
        // The server expects parameter keys prefixed with `param__`.
        // Callers pass plain variable names (e.g. `svc`); we apply the
        // prefix here so the SDK surface mirrors the MPL `$svc` syntax.
        let params = opts
            .params
            .into_iter()
            .map(|(k, v)| (format!("param__{k}"), v))
            .collect();
        let body = MplQueryRequest {
            apl: mpl.to_string(),
            start_time: start,
            end_time: end,
            query_edge_deployment: opts.edge_deployment,
            params,
        };

        let resp = self
            .http_client
            .post_with_headers("/v1/query/_mpl", &body, accept_header(ACCEPT_METRICS_V2))
            .await?;

        let trace_id = resp
            .headers()
            .get("x-axiom-trace-id")
            .map(|s| s.to_str())
            .transpose()
            .map_err(|_e| Error::InvalidTraceId)?
            .map(ToString::to_string);

        // Use the body-snippet decoder: the `_mpl` response shape is the
        // most likely place for a server/client schema drift, so on
        // failure we want the user to see what the server actually
        // returned instead of a generic "error decoding response body".
        let mut result: MetricsQueryResponse = resp.json_with_body_snippet().await?;
        result.trace_id = trace_id;
        Ok(result)
    }
}

/// Per-call options for [`Client::query`].
#[derive(Debug, Clone, Default)]
#[must_use]
pub struct MplQueryOptions {
    /// Override the edge deployment string sent as `queryEdgeDeployment`
    /// (e.g. `"cloud.eu-central-1.aws"`). When `None` the server picks
    /// a default for the configured edge URL.
    pub edge_deployment: Option<String>,
    /// MPL `param` values, keyed by the variable name (no leading `$`,
    /// no `param__` prefix — the SDK adds it). Values are forwarded
    /// verbatim as **MPL literals**, so string literals must include their
    /// own quotes (e.g. `"\"frontend\""`, not `"frontend"`); see the MPL
    /// spec for per-type literal syntax. An empty map omits the field
    /// entirely.
    pub params: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
struct FindMetricsRequest {
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MplQueryRequest {
    apl: String,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    query_edge_deployment: Option<String>,
    // Wire name is `params`, not `queryParams`. Keys must already carry
    // the `param__` prefix (applied by `Client::query`).
    #[serde(rename = "params", skip_serializing_if = "BTreeMap::is_empty")]
    params: BTreeMap<String, String>,
}

/// Build an `Accept`-only header map.
///
/// Only ever called with our two static media-type literals, so the
/// `HeaderValue` conversion can't fail.
fn accept_header(value: &'static str) -> HeaderMap {
    let mut headers = HeaderMap::with_capacity(1);
    headers.insert(ACCEPT, HeaderValue::from_static(value));
    headers
}

/// Percent-encode a URL path segment using the RFC3986 unreserved set.
/// Used for `metric` and `tag` identifiers that flow into the path; the
/// `?start=...&end=...` query string is built via `serde_qs`.
fn encode_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                // `write!` on a `String` is infallible.
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
    out
}
