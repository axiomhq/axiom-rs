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
async fn spec() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(OPTIONS)
            .path("/v1/query/_mpl")
            .header("accept", "text/markdown");
        then.status(200)
            .header("content-type", "text/markdown")
            .body("# MPL Spec\n\noperators: align, group by, where");
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_edge_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let spec = client.metrics().spec().await?;
    assert!(spec.starts_with("# MPL Spec"));
    mock.assert_hits_async(1).await;
    Ok(())
}

#[tokio::test]
async fn list_metrics() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let (start, end) = one_hour();
    // Lock down the wire shape: serde_qs renders timestamps via
    // `DateTime<Utc>`'s default serializer, which uses RFC3339 with `Z`.
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/query/metrics/info/datasets/ds/metrics")
            .header("accept", "application/vnd.metrics-info.v2+json")
            .query_param("start", "2026-01-01T11:00:00Z")
            .query_param("end", "2026-01-01T12:00:00Z");
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
    mock.assert_hits_async(1).await;
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
    mock.assert_hits_async(1).await;
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
    mock.assert_hits_async(1).await;
    Ok(())
}

#[tokio::test]
async fn tags_path_segment_is_percent_encoded() -> Result<(), Box<dyn std::error::Error>> {
    // Metric / tag identifiers flow into URL path segments. Anything
    // outside the RFC3986 unreserved set must be percent-encoded so the
    // server sees the original name on the other side. Slash is the
    // dangerous one: an unencoded `/` would split the path.
    let server = MockServer::start();
    let (start, end) = one_hour();
    let mock = server.mock(|when, then| {
        when.method(GET).path(
            "/v1/query/metrics/info/datasets/ds/metrics/weird%2Fmetric%20name/tags",
        );
        then.status(200).json_body(json!(["k"]));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_edge_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let tags = client
        .metrics()
        .tags("ds", "weird/metric name", start, end)
        .await?;
    assert_eq!(tags, vec!["k"]);
    mock.assert_hits_async(1).await;
    Ok(())
}

#[tokio::test]
async fn dataset_tags() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let (start, end) = one_hour();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/query/metrics/info/datasets/ds/tags");
        then.status(200)
            .json_body(json!(["env", "region", "service.name"]));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_edge_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let tags = client.metrics().dataset_tags("ds", start, end).await?;
    assert_eq!(tags, vec!["env", "region", "service.name"]);
    mock.assert_hits_async(1).await;
    Ok(())
}

#[tokio::test]
async fn dataset_tag_values() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let (start, end) = one_hour();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v1/query/metrics/info/datasets/ds/tags/service.name/values");
        then.status(200)
            .json_body(json!(["frontend", "checkout", "payments"]));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_edge_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let values = client
        .metrics()
        .dataset_tag_values("ds", "service.name", start, end)
        .await?;
    assert_eq!(values, vec!["frontend", "checkout", "payments"]);
    mock.assert_hits_async(1).await;
    Ok(())
}

#[tokio::test]
async fn find_metrics() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let (start, end) = one_hour();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/query/metrics/info/datasets/ds/metrics")
            .json_body(json!({ "value": "frontend" }));
        then.status(200)
            .json_body(json!(["http.server.duration", "http.server.requests"]));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_edge_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let metrics = client
        .metrics()
        .find_metrics("ds", "frontend", start, end)
        .await?;
    assert_eq!(
        metrics,
        vec!["http.server.duration", "http.server.requests"]
    );
    mock.assert_hits_async(1).await;
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
    mock.assert_hits_async(1).await;
    Ok(())
}

#[tokio::test]
async fn query_mpl_params_wire_shape() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let (start, end) = one_hour();
    // Assert the outgoing JSON body:
    //   * uses `params` (not `queryParams`)
    //   * prefixes every key with `param__` (no unprefixed keys leak
    //     through)
    //
    // We parse the body in a matcher closure so we can assert
    // *absence* of the wrong keys, not just presence of the right ones
    // — `json_body_partial` alone would happily pass if both `params`
    // and `queryParams` were sent.
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/query/_mpl")
            .matches(|req| {
                let bytes = match req.body.as_deref() {
                    Some(b) => b,
                    None => return false,
                };
                let body: serde_json::Value = match serde_json::from_slice(bytes) {
                    Ok(v) => v,
                    Err(_) => return false,
                };
                let obj = match body.as_object() {
                    Some(o) => o,
                    None => return false,
                };
                // Wrong field name must not appear.
                if obj.contains_key("queryParams") {
                    return false;
                }
                let params = match obj.get("params").and_then(|v| v.as_object()) {
                    Some(p) => p,
                    None => return false,
                };
                // Prefixed keys present, raw keys absent.
                params.contains_key("param__svc")
                    && params.contains_key("param__window")
                    && !params.contains_key("svc")
                    && !params.contains_key("window")
                    && params.get("param__svc").and_then(|v| v.as_str())
                        == Some("\"frontend\"")
                    && params.get("param__window").and_then(|v| v.as_str()) == Some("5m")
            });
        then.status(200).json_body(json!({ "series": [] }));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_edge_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let mut params = std::collections::BTreeMap::new();
    params.insert("svc".to_string(), "\"frontend\"".to_string());
    params.insert("window".to_string(), "5m".to_string());
    let opts = MplQueryOptions {
        params,
        ..Default::default()
    };
    let _ = client
        .metrics()
        .query("param $svc: string; param $window: Duration; _", start, end, opts)
        .await?;
    mock.assert_hits_async(1).await;
    Ok(())
}
