//! Types describing the dashboards `v2` API.
//!
//! The wire shape is intentionally permissive: every type carries an `extras`
//! bucket capturing unknown fields, so an SDK build that pre-dates a server
//! schema change can still round-trip a dashboard through `get` → `put`
//! without silently dropping new fields.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One dashboard resource as returned by the `v2` API.
///
/// `uid` is the path-friendly identifier used by [`Client::get`],
/// [`Client::put`], and [`Client::delete`]. `version` is the
/// server-assigned monotonic counter you must round-trip on writes unless
/// you explicitly opt in to overwrite semantics.
///
/// [`Client::get`]: super::Client::get
/// [`Client::put`]: super::Client::put
/// [`Client::delete`]: super::Client::delete
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Dashboard {
    /// Path-friendly dashboard identifier (used by the URI).
    pub uid: String,
    /// Numeric, internal identifier. Distinct from [`Dashboard::uid`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// When the dashboard was last persisted.
    #[serde(rename = "updatedAt", default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    /// Who last persisted the dashboard.
    #[serde(rename = "updatedBy", default, skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
    /// Server-assigned monotonic version. Required as `version` on the
    /// next `PUT` unless `overwrite=true` is sent; absent for hand-authored
    /// payloads that have never been persisted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    /// The nested document — charts, layout, time window, and everything
    /// the server doesn't surface at the resource level.
    #[serde(default)]
    pub dashboard: DashboardDocument,
}

impl Dashboard {
    /// Convenience accessor for `dashboard.name`.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.dashboard.name.as_deref()
    }

    /// Convenience accessor for `dashboard.description`.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.dashboard.description.as_deref()
    }
}

/// Body of a dashboard create or update request.
///
/// Use [`Client::create`] or [`Client::put`] rather than building this
/// directly; the helpers fill in `uid`, `version`, and `overwrite` so callers
/// don't have to remember the version-check semantics.
///
/// [`Client::create`]: super::Client::create
/// [`Client::put`]: super::Client::put
#[derive(Debug, Clone, Serialize)]
pub struct UpsertRequest<'a> {
    /// The dashboard document being created or replaced.
    pub dashboard: &'a DashboardDocument,
    /// Expected current version on the server. Sent on update to opt into
    /// the server's optimistic version check; omitted on create.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    /// When `true`, the server skips the version check entirely.
    #[serde(default, skip_serializing_if = "is_false")]
    pub overwrite: bool,
    /// Desired `uid`. Optional on create; the helper sets it for `PUT`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<&'a str>,
    /// Free-form description of the change, persisted as audit metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<&'a str>,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(b: &bool) -> bool {
    !*b
}

/// Response envelope shared by `POST /v2/dashboards` and
/// `PUT /v2/dashboards/uid/{uid}`.
#[derive(Debug, Clone, Deserialize)]
pub struct DashboardWriteResponse {
    /// Whether the call created a new dashboard or updated an existing one.
    pub status: DashboardWriteStatus,
    /// `true` when the server skipped the version check.
    #[serde(default)]
    pub overwritten: Option<bool>,
    /// Resulting resource, including the bumped `version`.
    pub dashboard: Dashboard,
}

/// Distinguishes the two outcomes of a successful upsert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DashboardWriteStatus {
    /// A new dashboard was created.
    Created,
    /// An existing dashboard was updated.
    Updated,
}

/// Nested dashboard document.
///
/// The server's spec is `additionalProperties: false` on this object, which
/// means we have to round-trip exactly what came in or `PUT` will reject the
/// payload. The [`DashboardDocument::extras`] bucket is the safety net for
/// fields the SDK doesn't yet model.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DashboardDocument {
    /// Human-readable dashboard name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional one-line description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Charts on the dashboard. Order is preserved.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub charts: Vec<Chart>,
    /// Grid placement for each chart. The server's coordinate system is
    /// 12 columns wide; `y` may be `null` to auto-stack.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layout: Vec<LayoutItem>,
    /// Time window start (literal, e.g. `"now-1h"`).
    #[serde(
        rename = "timeWindowStart",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub time_window_start: Option<String>,
    /// Time window end (literal, e.g. `"now"`).
    #[serde(
        rename = "timeWindowEnd",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub time_window_end: Option<String>,
    /// Every other field the server returned. Preserved verbatim for
    /// round-tripping. Includes (for example) `owner`, `refreshTime`,
    /// `schemaVersion`, `against`, `againstTimestamp`, `uid`, …
    #[serde(flatten)]
    pub extras: serde_json::Map<String, serde_json::Value>,
}

/// One chart on a dashboard.
///
/// Each variant carries the shared [`ChartBase`] fields plus any
/// chart-specific keys preserved verbatim in [`ChartBase::extras`].
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Chart {
    /// Time-series line/area chart.
    TimeSeries(ChartBase),
    /// Heatmap chart.
    Heatmap(ChartBase),
    /// Log stream panel.
    LogStream(ChartBase),
    /// Pie chart.
    Pie(ChartBase),
    /// Scatter plot.
    Scatter(ChartBase),
    /// Tabular panel.
    Table(ChartBase),
    /// Top-K bar chart.
    TopK(ChartBase),
    /// Single-statistic panel.
    Statistic(ChartBase),
    /// Static markdown/HTML note panel.
    Note(ChartBase),
}

impl Chart {
    /// Borrow the shared chart fields regardless of variant.
    #[must_use]
    pub fn base(&self) -> &ChartBase {
        match self {
            Chart::TimeSeries(b)
            | Chart::Heatmap(b)
            | Chart::LogStream(b)
            | Chart::Pie(b)
            | Chart::Scatter(b)
            | Chart::Table(b)
            | Chart::TopK(b)
            | Chart::Statistic(b)
            | Chart::Note(b) => b,
        }
    }

    /// Borrow the shared chart fields mutably regardless of variant.
    #[must_use]
    pub fn base_mut(&mut self) -> &mut ChartBase {
        match self {
            Chart::TimeSeries(b)
            | Chart::Heatmap(b)
            | Chart::LogStream(b)
            | Chart::Pie(b)
            | Chart::Scatter(b)
            | Chart::Table(b)
            | Chart::TopK(b)
            | Chart::Statistic(b)
            | Chart::Note(b) => b,
        }
    }

    /// Human-readable type name, exactly as it appears on the wire.
    #[must_use]
    pub fn type_str(&self) -> &'static str {
        match self {
            Chart::TimeSeries(_) => "TimeSeries",
            Chart::Heatmap(_) => "Heatmap",
            Chart::LogStream(_) => "LogStream",
            Chart::Pie(_) => "Pie",
            Chart::Scatter(_) => "Scatter",
            Chart::Table(_) => "Table",
            Chart::TopK(_) => "TopK",
            Chart::Statistic(_) => "Statistic",
            Chart::Note(_) => "Note",
        }
    }
}

/// Fields shared by every chart variant.
///
/// The `query` shape differs per variant (e.g. `TimeSeriesChartQuery` vs
/// `SimpleChartQuery`), so it is parked as raw JSON for the SDK consumer
/// to interpret.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChartBase {
    /// Stable per-dashboard chart identifier; matches [`LayoutItem::i`].
    pub id: String,
    /// Display name shown above the chart.
    #[serde(default)]
    pub name: Option<String>,
    /// Raw query specification for this chart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<serde_json::Value>,
    /// Any other fields the server returned for this chart variant
    /// (e.g. `tableSettings`, `colorScheme`). Preserved verbatim for
    /// round-trip.
    #[serde(flatten)]
    pub extras: serde_json::Map<String, serde_json::Value>,
}

/// Grid placement for a chart, keyed by the chart's id.
///
/// The grid is 12 columns wide (`x` ∈ `0..=11`); `y` may be `None` to let the
/// server auto-stack from the top.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LayoutItem {
    /// Matches [`ChartBase::id`].
    pub i: String,
    /// Leftmost column occupied by the tile, in the range `0..=11`.
    pub x: u32,
    /// Topmost row occupied by the tile. `None` lets the server auto-stack.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<u32>,
    /// Tile width in columns.
    pub w: u32,
    /// Tile height in rows.
    pub h: u32,
    /// Any other fields preserved verbatim for round-trip.
    #[serde(flatten)]
    pub extras: serde_json::Map<String, serde_json::Value>,
}
