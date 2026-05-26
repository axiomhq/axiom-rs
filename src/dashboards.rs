//! Create, read, update, and delete Axiom dashboards.
//!
//! You're probably looking for the [`Client`].
//!
//! # Examples
//! ```no_run
//! use axiom_rs::{Client, Error};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     let client = Client::new()?;
//!
//!     // Fetch every dashboard the token can see.
//!     let dashboards = client.dashboards().list().await?;
//!     for d in &dashboards {
//!         println!("{}\t{}", d.uid, d.name().unwrap_or("(unnamed)"));
//!     }
//!
//!     // Load a single dashboard by uid.
//!     if let Some(first) = dashboards.first() {
//!         let dashboard = client.dashboards().get(&first.uid).await?;
//!         println!("loaded version {:?}", dashboard.version);
//!     }
//!
//!     Ok(())
//! }
//! ```
mod client;
mod model;
#[cfg(test)]
mod tests;

pub use client::Client;
pub use model::{
    Chart, ChartBase, CreateOptions, Dashboard, DashboardDocument, DashboardWriteResponse,
    DashboardWriteStatus, KnownChart, LayoutItem, UpsertOptions, UpsertRequest,
};
