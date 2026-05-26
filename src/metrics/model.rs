//! Types describing the metrics-info and `_mpl` endpoints.
//!
//! The metrics API ships its own JSON encodings — different `Accept`
//! headers select between a flat tag list and the richer `v2` info shape.
//! The structs here cover both surfaces.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Metadata for a single metric, as returned by the metrics-info endpoint.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricInfo {
    /// Metric type (e.g. `"counter"`, `"gauge"`, `"histogram"`). Renamed
    /// from the wire `"type"` field to avoid the Rust keyword.
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    /// Temporality reported by the producer (`"delta"`, `"cumulative"`, …).
    #[serde(default)]
    pub temporality: Option<String>,
    /// Unit declared by the producer, if any.
    #[serde(default)]
    pub unit: Option<String>,
}

/// Response body of `POST /v1/query/_mpl`.
///
/// `trace_id` is populated from the response's `x-axiom-trace-id` header
/// — it is **not** part of the JSON body, so serde leaves it `None` on
/// decode and the SDK fills it in.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MetricsQueryResponse {
    /// Per-series result. Empty when the query matched nothing.
    #[serde(default)]
    pub series: Vec<MetricsSeries>,
    /// Trace identifier extracted from the response headers, when present.
    #[serde(skip)]
    pub trace_id: Option<String>,
}

/// One time-series in an MPL response.
#[derive(Debug, Clone, Deserialize)]
pub struct MetricsSeries {
    /// Source metric name.
    pub metric: String,
    /// Tag bindings that uniquely identify this series (e.g.
    /// `{"service": "api", "code": "200"}`).
    #[serde(default)]
    pub tags: HashMap<String, String>,
    /// First-sample timestamp in unix milliseconds.
    pub start: i64,
    /// Step size between samples in milliseconds.
    pub resolution: u64,
    /// Sample values; gaps are `None`. Indexed by
    /// `start + i * resolution`.
    #[serde(default)]
    pub data: Vec<Option<f64>>,
}
