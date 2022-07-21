# axiom-rs

[![CI](https://github.com/axiomhq/axiom-rs/workflows/CI/badge.svg)](https://github.com/axiomhq/axiom-rs/actions?query=workflow%3ACI)
[![crates.io](https://img.shields.io/crates/v/axiom-rs.svg)](https://crates.io/crates/axiom-rs)
[![docs.rs](https://docs.rs/axiom-rs/badge.svg)](https://docs.rs/axiom-rs/)
[![License](https://img.shields.io/crates/l/axiom-rs)](LICENSE-APACHE)

The Rust SDK for [Axiom](https://axiom.co) — manage datasets, ingest and query 
data all from your Rust project.

## Install

Add the following to your Cargo.toml:

```toml
[dependencies]
axiom-rs = "0.2"
```

## Get started

This library uses [Tokio](https://tokio.rs) by default, so your `Cargo.toml` 
could look like this:

```toml
[dependencies]
axiom-rs = "0.2"
tokio = "1"
```

<details>
<summary>Usage with async-std</summary>

If you want to use [async-std](https://async.rs/), you need to set some 
features:

```toml
[dependencies] 
axiom-rs = { version = "0.2", default-features = false, features = ["async-std"] }
async-std = "1"
```

</details>

And your `src/main.rs` like this:

```rust
use axiom_rs::Client;

#[tokio::main] // or #[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let client = Client::new()?;
  let datasets = client.datasets.list().await?;
  println!("{:?}", datasets);
  Ok(())
}
```

> **Note**: The `Client` constructor uses `AXIOM_TOKEN` and other parameters
  from your environment by default. See the
  [`Client` documentation](https://docs.rs/axiom-rs/struct.Client.html)
  for other options.

## Optional Features

The following are a list of
[Cargo features](https://doc.rust-lang.org/stable/cargo/reference/features.html#the-features-section)
that can be enabled or disabled:

- **default-tls** _(enabled by default)_: Provides TLS support to connect
  over HTTPS.
- **native-tls**: Enables TLS functionality provided by `native-tls`.
- **rustls-tls**: Enables TLS functionality provided by `rustls`.
- **tokio** _(enabled by default)_: Enables the usage with the `tokio` runtime.
- **async-std** : Enables the usage with the `async-std` runtime.

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
