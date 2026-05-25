#![cfg(test)]

use chrono::{TimeZone, Utc};
use httpmock::prelude::*;
use serde_json::json;

use crate::{metrics::client::MplQueryOptions, Client};

fn one_hour() -> (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) {
    let end = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
    let start = end - chrono::Duration::hours(1);
    (start, end)
}

#[tokio::test]
async fn list_metrics() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let (start, end) = one_hour();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/query/metrics/info/datasets/ds/metrics")
            .header("accept", "application/vnd.metrics-info.v2+json");
        then.status(200).json_body(json!({
            "http_requests_total": { "type": "counter", "temporality": "delta", "unit": null }
        }));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_edge_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let info = client.metrics().list("ds", start, end).await?;
    assert_eq!(info.len(), 1);
    assert_eq!(
        info.get("http_requests_total")
            .and_then(|m| m.kind.as_deref()),
        Some("counter")
    );
    mock.assert();
    Ok(())
}

#[tokio::test]
async fn list_tags() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let (start, end) = one_hour();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/query/metrics/info/datasets/ds/metrics/http_requests_total/tags");
        then.status(200).json_body(json!(["code", "service"]));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_edge_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let tags = client
        .metrics()
        .tags("ds", "http_requests_total", start, end)
        .await?;
    assert_eq!(tags, vec!["code", "service"]);
    mock.assert();
    Ok(())
}

#[tokio::test]
async fn tag_values() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let (start, end) = one_hour();
    let mock = server.mock(|when, then| {
        when.method(GET).path(
            "/v1/query/metrics/info/datasets/ds/metrics/http_requests_total/tags/code/values",
        );
        then.status(200).json_body(json!(["200", "404", "500"]));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_edge_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let values = client
        .metrics()
        .tag_values("ds", "http_requests_total", "code", start, end)
        .await?;
    assert_eq!(values, vec!["200", "404", "500"]);
    mock.assert();
    Ok(())
}

#[tokio::test]
async fn query_mpl() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let (start, end) = one_hour();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/query/_mpl")
            .header("accept", "application/json+metrics.v2");
        then.status(200)
            .header("x-axiom-trace-id", "trace-abc")
            .json_body(json!({
                "series": [
                    {
                        "metric": "http_requests_total",
                        "tags": { "code": "200" },
                        "start": 1_700_000_000_000_i64,
                        "resolution": 60_000_u64,
                        "data": [1.0, 2.0, null, 4.0]
                    }
                ]
            }));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_edge_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let opts = MplQueryOptions {
        edge_deployment: Some("cloud.eu-central-1.aws".into()),
        ..Default::default()
    };
    let res = client
        .metrics()
        .query("metric('http_requests_total')", start, end, opts)
        .await?;
    assert_eq!(res.series.len(), 1);
    assert_eq!(res.series[0].metric, "http_requests_total");
    assert_eq!(res.series[0].data.len(), 4);
    assert_eq!(res.trace_id.as_deref(), Some("trace-abc"));
    mock.assert();
    Ok(())
}
