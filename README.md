<!---
Keep this page in sync with https://github.com/axiomhq/docs/blob/main/guides/rust.mdx
-->

# axiom-rs

<a href="https://axiom.co">
<picture>
  <source media="(prefers-color-scheme: dark) and (min-width: 600px)" srcset="https://axiom.co/assets/github/axiom-github-banner-light-vertical.svg">
  <source media="(prefers-color-scheme: light) and (min-width: 600px)" srcset="https://axiom.co/assets/github/axiom-github-banner-dark-vertical.svg">
  <source media="(prefers-color-scheme: dark) and (max-width: 599px)" srcset="https://axiom.co/assets/github/axiom-github-banner-light-horizontal.svg">
  <img alt="Axiom.co banner" src="https://axiom.co/assets/github/axiom-github-banner-dark-horizontal.svg" align="right">
</picture>
</a>
&nbsp;

[![docs.rs](https://docs.rs/axiom-rs/badge.svg)](https://docs.rs/axiom-rs/)
[![build](https://img.shields.io/github/actions/workflow/status/axiomhq/axiom-rs/ci.yaml?branch=main&ghcache=unused)](https://github.com/axiomhq/axiom-rs/actions?query=workflow%3ACI)
[![crates.io](https://img.shields.io/crates/v/axiom-rs.svg)](https://crates.io/crates/axiom-rs)
[![License](https://img.shields.io/crates/l/axiom-rs)](LICENSE-APACHE)

[Axiom](https://axiom.co) unlocks observability at any scale.

- **Ingest with ease, store without limits:** Axiom's next-generation datastore
  enables ingesting petabytes of data with ultimate efficiency. Ship logs from
  Kubernetes, AWS, Azure, Google Cloud, DigitalOcean, Nomad, and others.
- **Query everything, all the time:** Whether DevOps, SecOps, or EverythingOps,
  query all your data no matter its age. No provisioning, no moving data from
  cold/archive to "hot", and no worrying about slow queries. All your data, all.
  the. time.
- **Powerful dashboards, for continuous observability:** Build dashboards to
  collect related queries and present information that's quick and easy to
  digest for you and your team. Dashboards can be kept private or shared with
  others, and are the perfect way to bring together data from different sources.

For more information check out the
[official documentation](https://axiom.co/docs) and our
[community Discord](https://axiom.co/discord).

## Prerequisites

- [Create an Axiom account](https://app.axiom.co/register).
- [Create a dataset in Axiom](https://axiom.co/docs/reference/datasets) where you send your data.
- [Create an API token in Axiom](https://axiom.co/docs/reference/tokens) with permissions to update the dataset you have created.

## Install SDK

Add the following to your `Cargo.toml`:

```toml
[dependencies]
axiom-rs = "VERSION"
```

Replace `VERSION` with the latest version number specified on the [GitHub Releases](https://github.com/axiomhq/axiom-rs/releases) page. For example, `0.11.0`.

If you use the [Axiom CLI](https://axiom.co/reference/cli), run `eval $(axiom config export -f)` to configure your environment variables. Otherwise, [create an API token](https://axiom.co/reference/tokens) and export it as `AXIOM_TOKEN`.

## Use client

```rust,no_run
use axiom_rs::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build your client by providing a personal token and an org id:
    let client = Client::builder()
        .with_token("API_TOKEN")
        .build()?;

    // Alternatively, auto-configure the client from the environment variable AXIOM_TOKEN:
    let client = Client::new()?;

    client.datasets().create("DATASET_NAME", "").await?;

    client
        .ingest(
            "DATASET_NAME",
            vec![json!({
                "foo": "bar",
            })],
        )
        .await?;

    let res = client
        .query(r#"['DATASET_NAME'] | where foo == "bar" | limit 100"#, None)
        .await?;
    println!("{:?}", res);

    client.datasets().delete("DATASET_NAME").await?;
    Ok(())
}
```

For more examples, see the [examples folder](https://github.com/axiomhq/axiom-rs/tree/main/examples).

## Optional features

You can use the [Cargo features](https://doc.rust-lang.org/stable/cargo/reference/features.html#the-features-section):

- `default-tls`: Provides TLS support to connect over HTTPS. Enabled by default.
- `native-tls`: Enables TLS functionality provided by `native-tls`.
- `rustls-tls`: Enables TLS functionality provided by `rustls`.
- `tokio`: Enables usage with the `tokio` runtime. Enabled by default.
- `async-std`: Enables usage with the `async-std` runtime.