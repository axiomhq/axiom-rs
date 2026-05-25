//! Discover metrics, browse their tags, and run MPL queries against the
//! Axiom edge endpoint.
//!
//! You're probably looking for the [`Client`].
//!
//! # Examples
//! ```no_run
//! use axiom_rs::{Client, Error};
//! use chrono::{Duration, Utc};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     let client = Client::new()?;
//!     let end = Utc::now();
//!     let start = end - Duration::hours(1);
//!
//!     // What metrics live in this dataset over the last hour?
//!     let info = client.metrics().list("my-metrics-dataset", start, end).await?;
//!     for (name, meta) in &info {
//!         println!("{name}\t{:?}\t{:?}", meta.kind, meta.unit);
//!     }
//!
//!     // Run an MPL query.
//!     let res = client
//!         .metrics()
//!         .query("metric('http_requests_total') | rate(1m)", start, end, None)
//!         .await?;
//!     for series in &res.series {
//!         println!("{}\t{} points", series.metric, series.data.len());
//!     }
//!
//!     Ok(())
//! }
//! ```
mod client;
mod model;
#[cfg(test)]
mod tests;

pub use client::{Client, MplQueryOptions};
pub use model::{MetricInfo, MetricsQueryResponse, MetricsSeries};
