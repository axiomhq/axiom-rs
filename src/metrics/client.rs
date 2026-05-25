use std::{collections::BTreeMap, fmt::Debug as FmtDebug};

use std::fmt::Write;

use ::http::header::{HeaderMap, HeaderValue, ACCEPT};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tracing::instrument;

use crate::{
    error::Result,
    http,
    metrics::model::{MetricInfo, MetricsQueryResponse},
};

/// Accept header for the metrics-info endpoint.
const ACCEPT_METRICS_INFO_V2: &str = "application/vnd.metrics-info.v2+json";
/// Accept header for the `_mpl` query endpoint.
const ACCEPT_METRICS_V2: &str = "application/json+metrics.v2";

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
    pub async fn list<D>(
        &self,
        dataset: D,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<BTreeMap<String, MetricInfo>>
    where
        D: Into<String> + FmtDebug,
    {
        let dataset = dataset.into();
        let path = format!(
            "/v1/query/metrics/info/datasets/{}/metrics?start={}&end={}",
            dataset,
            encode_rfc3339(start),
            encode_rfc3339(end),
        );
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
    pub async fn tags<D, M>(
        &self,
        dataset: D,
        metric: M,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<String>>
    where
        D: Into<String> + FmtDebug,
        M: Into<String> + FmtDebug,
    {
        let dataset = dataset.into();
        let metric = metric.into();
        let path = format!(
            "/v1/query/metrics/info/datasets/{}/metrics/{}/tags?start={}&end={}",
            dataset,
            url_segment(&metric),
            encode_rfc3339(start),
            encode_rfc3339(end),
        );
        self.http_client.get(path).await?.json().await
    }

    /// List the observed values for `tag` of `(dataset, metric)` between
    /// `start` and `end`.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the response cannot be
    /// deserialised.
    #[instrument(skip(self))]
    pub async fn tag_values<D, M, T>(
        &self,
        dataset: D,
        metric: M,
        tag: T,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<String>>
    where
        D: Into<String> + FmtDebug,
        M: Into<String> + FmtDebug,
        T: Into<String> + FmtDebug,
    {
        let dataset = dataset.into();
        let metric = metric.into();
        let tag = tag.into();
        let path = format!(
            "/v1/query/metrics/info/datasets/{}/metrics/{}/tags/{}/values?start={}&end={}",
            dataset,
            url_segment(&metric),
            url_segment(&tag),
            encode_rfc3339(start),
            encode_rfc3339(end),
        );
        self.http_client.get(path).await?.json().await
    }

    /// Run an MPL query against the edge endpoint.
    ///
    /// The request field name is `apl` for historical reasons; the payload
    /// is MPL, not APL.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the response cannot be
    /// deserialised.
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
        let body = MplQueryRequest {
            apl: mpl.to_string(),
            start_time: start,
            end_time: end,
            query_edge_deployment: opts.edge_deployment,
            query_params: opts.params,
        };

        let resp = self
            .http_client
            .post_with_headers("/v1/query/_mpl", &body, accept_header(ACCEPT_METRICS_V2))
            .await?;

        let trace_id = resp
            .headers()
            .get("x-axiom-trace-id")
            .or_else(|| resp.headers().get("traceparent"))
            .and_then(|v| v.to_str().ok())
            .map(ToString::to_string);

        let mut result: MetricsQueryResponse = resp.json().await?;
        result.trace_id = trace_id;
        Ok(result)
    }
}

/// Per-call options for [`Client::query`].
#[derive(Debug, Clone, Default)]
pub struct MplQueryOptions {
    /// Override the edge deployment string sent as `queryEdgeDeployment`
    /// (e.g. `"cloud.eu-central-1.aws"`). When `None` the server picks
    /// a default for the configured edge URL.
    pub edge_deployment: Option<String>,
    /// User-supplied MPL `param` values; serialised as `queryParams`. Use
    /// an empty map to omit the field entirely.
    pub params: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MplQueryRequest {
    apl: String,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    query_edge_deployment: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    query_params: BTreeMap<String, String>,
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

/// Format a timestamp the way the metrics-info endpoint requires:
/// strict RFC3339, percent-encoded so `:` and `+` round-trip safely.
fn encode_rfc3339(t: DateTime<Utc>) -> String {
    url_segment(&t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

/// Minimal percent-encoding for a URL path/query segment.
fn url_segment(s: &str) -> String {
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
