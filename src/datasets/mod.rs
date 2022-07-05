//! Manage datasets, ingest data and query it.
//!
//! You're probably looking for the [`Client`].
//!
//! # Examples
//! ```
//! use axiom_rs::{Client, Error};
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     let client = Client::new()?;
//!
//!     client.datasets.create("my-dataset", "").await?;
//!
//!     client.datasets.ingest("my-dataset", vec![
//!       json!({
//!         "foo": "bar",
//!         "bar": "baz"
//!       }),
//!     ]).await?;
//!
//!     let res = client.datasets.apl_query("['my-dataset'] | count", None).await?;
//!     assert_eq!(1, res.status.rows_matched);
//!
//!     client.datasets.delete("my-dataset").await?;
//!
//!     Ok(())
//! }
//! ```
mod client;
mod model;

pub use client::Client;
pub use model::*;
