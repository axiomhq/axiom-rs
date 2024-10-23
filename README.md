# axiom-rs [![docs.rs](https://docs.rs/axiom-rs/badge.svg)](https://docs.rs/axiom-rs/) [![build](https://img.shields.io/github/actions/workflow/status/axiomhq/axiom-rs/ci.yaml?branch=main&ghcache=unused)](https://github.com/axiomhq/axiom-rs/actions?query=workflow%3ACI) [![crates.io](https://img.shields.io/crates/v/axiom-rs.svg)](https://crates.io/crates/axiom-rs) [![License](https://img.shields.io/crates/l/axiom-rs)](LICENSE-APACHE)

```rust,no_run
use axiom_rs::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Auto-configure the client from the environment variable AXIOM_TOKEN:
    let client = Client::new()?;

    client
        .ingest(
            "DATASET_NAME",
            vec![json!({
                "foo": "bar",
            })],
        )
        .await?;
    let _res = client
        .query(r#"['DATASET_NAME'] | where foo == "bar" | limit 100"#, None)
        .await?;
    Ok(())
}
```

## Install

```sh
cargo add axiom-rs
```

## Optional features

You can use the [Cargo features](https://doc.rust-lang.org/stable/cargo/reference/features.html#the-features-section):

- `default-tls`: Provides TLS support to connect over HTTPS. Enabled by default.
- `native-tls`: Enables TLS functionality provided by `native-tls`.
- `rustls-tls`: Enables TLS functionality provided by `rustls`.
- `tokio`: Enables usage with the `tokio` runtime. Enabled by default.
- `async-std`: Enables usage with the `async-std` runtime.

## Documentation

Read documentation on [axiom.co/docs/guides/rust](https://axiom.co/docs/guides/rust).

## License

[MIT](LICENSE-MIT) or [Apache](LICENSE-APACHE)
