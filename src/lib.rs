//! The Rust SDK for Axiom.
//!
//! If you're just getting started, take a look at the [`Client`].
//! It contains all methods you'll need to interact with the API.
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
//!     // Create a dataset called my-dataset
//!     let dataset = client.datasets.create("my-dataset", "a description").await?;
//!
//!     // Ingest one event
//!     client.ingest(&dataset.name, vec![
//!         json!({"foo": "bar"})
//!     ]).await?;
//!
//!     // Query the dataset
//!     let query_res = client.query(r#"['my-dataset']"#, None).await?;
//!     dbg!(query_res.matches);
//!
//!     // Delete the dataset
//!     client.datasets.delete(dataset.name).await?;
//!
//!     Ok(())
//! }
//! ```
pub mod client;
pub mod error;
mod http;
#[cfg(not(feature = "blocking"))]
mod http_async;
#[cfg(feature = "blocking")]
mod http_blocking;
pub mod limits;
mod serde;

pub mod datasets;
pub mod users;

pub use client::Client;
pub use error::Error;

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

#[cfg(all(feature = "tokio", feature = "async-std"))]
compile_error!("Feature \"tokio\" and \"async-std\" cannot be enabled at the same time");

#[cfg(all(feature = "default-tls", feature = "native-tls"))]
compile_error!("Feature \"default-tls\" and \"native-tls\" cannot be enabled at the same time");

#[cfg(all(feature = "native-tls", feature = "rustls-tls"))]
compile_error!("Feature \"native-tls\" and \"rustls-tls\" cannot be enabled at the same time");

#[cfg(all(feature = "rustls-tls", feature = "default-tls"))]
compile_error!("Feature \"rustls-tls\" and \"default-tls\" cannot be enabled at the same time");

#[cfg(all(feature = "blocking", any(feature = "tokio", feature = "async-std")))]
compile_error!(
    "Feature \"blocking\" cannot be enabled at the same time as \"tokio\" or \"async-std\""
);

/// Returns true if the given acces token is a personal token.
fn is_personal_token<S: Into<String>>(token: S) -> bool {
    token.into().starts_with("xapt-")
}
