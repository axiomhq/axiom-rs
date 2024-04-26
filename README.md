# axiom-rs 

<img src="https://github.com/axiomhq/axiom-rs/assets/1725839/7cb313d2-96f1-4cb2-928a-3b2c301b57ac" align="right" width="300" />

[![docs.rs](https://docs.rs/axiom-rs/badge.svg)](https://docs.rs/axiom-rs/) [![build](https://img.shields.io/github/actions/workflow/status/axiomhq/axiom-rs/ci.yaml?branch=main&ghcache=unused)](https://github.com/axiomhq/axiom-rs/actions?query=workflow%3ACI) [![crates.io](https://img.shields.io/crates/v/axiom-rs.svg)](https://crates.io/crates/axiom-rs) [![License](https://img.shields.io/crates/l/axiom-rs)](LICENSE-APACHE)

The official Rust bindings for the Axiom API.

Add the following to your Cargo.toml:

```toml
[dependencies]
axiom-rs = "0.9"
```

If you use the [Axiom CLI](https://github.com/axiomhq/cli), run
`eval $(axiom config export -f)` to configure your environment variables.

Otherwise create a personal token in
[the Axiom settings](https://cloud.axiom.co/profile) and make note of
the organization ID from the settings page of the organization you want to
access.

Create and use a client like this:

```rust
use axiom_rs::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build your client by providing a personal token and an org id:
    let client = Client::builder()
        .with_token("my-token")
        .with_org_id("my-org")
        .build()?;

    // Alternatively you autoconfigure the client from the environment variables
    // AXIOM_TOKEN and AXIOM_ORG_ID:
    let client = Client::new()?;

    client.datasets.create("my-dataset", "").await?;

    client
        .ingest(
            "my-dataset",
            vec![json!({
                "foo": "bar",
            })],
        )
        .await?;

    let res = client
        .query(r#"['my-dataset'] | where foo == "bar" | limit 100"#, None)
        .await?;
    println!("{:?}", res);

    client.datasets.delete("my-dataset").await?;
    Ok(())
}
```

For further examples, head over to the [examples](examples) directory.

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
