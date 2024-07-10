//! Manage datasets, ingest data and query it.
//!
//! You're probably looking for the [`Client`].
//!
//! # Examples
//! ```no_run
//! use axiom_rs::{Client, Error, annotations::requests};
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     let client = Client::new()?;
//!
//!     let req = requests::Create::builder()
//!             .with_type("cake")?
//!             .with_datasets(vec!["snot".to_string(), "badger".to_string()])?
//!             .with_title("cookie")
//!             .build();
//!     client.annotations().create(req).await?;
//!
//!     let res = client.annotations().list(requests::List::default()).await?;
//!     assert_eq!(1, res.len());
//!
//!     client.annotations().delete(&res[1].id).await?;
//!
//!     Ok(())
//! }
//! ```
//!
mod client;
mod model;
pub mod requests;
#[cfg(test)]
mod tests;

pub use client::Client;
pub use model::Annotation;
