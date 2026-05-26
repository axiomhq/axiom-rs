use crate::{
    dashboards::model::{
        Chart, DashboardDocument, DashboardWriteStatus, KnownChart, UpsertOptions,
    },
    error::Error,
    limits, Client,
};
use chrono::{Duration, Utc};
use httpmock::prelude::*;
use serde_json::json;

#[test]
fn chart_unknown_variant_round_trips() {
    // A chart whose `type` the SDK doesn't model must deserialise into
    // `Chart::Unknown(_)` and round-trip back to the original JSON without
    // dropping any fields.
    let raw = json!({
        "type": "FutureChartKind",
        "id": "c-1",
        "name": "From the future",
        "futureSetting": { "answer": 42 }
    });
    let chart: Chart = serde_json::from_value(raw.clone()).expect("chart decode");
    match &chart {
        Chart::Unknown(v) => assert_eq!(
            v.get("type").and_then(|t| t.as_str()),
            Some("FutureChartKind")
        ),
        Chart::Known(k) => panic!("expected Unknown, got Known: {:?}", k),
    }
    assert_eq!(chart.type_str(), Some("FutureChartKind"));
    assert!(chart.base().is_none());
    let round_tripped = serde_json::to_value(&chart).expect("chart encode");
    assert_eq!(round_tripped, raw);
}

#[test]
fn chart_known_variant_round_trips() {
    let raw = json!({
        "type": "TimeSeries",
        "id": "c-2",
        "name": "Latency",
        "query": { "apl": "..." }
    });
    let chart: Chart = serde_json::from_value(raw.clone()).expect("chart decode");
    match &chart {
        Chart::Known(KnownChart::TimeSeries(base)) => assert_eq!(base.id, "c-2"),
        other => panic!("expected Known::TimeSeries, got {:?}", other),
    }
    assert_eq!(chart.type_str(), Some("TimeSeries"));
    let round_tripped = serde_json::to_value(&chart).expect("chart encode");
    assert_eq!(round_tripped, raw);
}

#[test]
fn chart_without_name_does_not_emit_null() {
    // Regression: `ChartBase.name` used to round-trip absent fields as
    // `"name": null`, which violates the dashboard document's
    // `additionalProperties: false` constraint on some shapes.
    let raw = json!({
        "type": "TimeSeries",
        "id": "c-3"
    });
    let chart: Chart = serde_json::from_value(raw.clone()).expect("chart decode");
    let round_tripped = serde_json::to_value(&chart).expect("chart encode");
    assert_eq!(round_tripped, raw);
    assert!(
        round_tripped.get("name").is_none(),
        "absent name must not serialise as null, got {}",
        round_tripped
    );
}

#[tokio::test]
async fn list_dashboards() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v2/dashboards")
            .query_param("limit", "1000");
        then.status(200).json_body(json!([
            {
                "uid": "dash-1",
                "version": 3,
                "dashboard": { "name": "Latency", "charts": [], "layout": [] }
            }
        ]));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let dashboards = client.dashboards().list().await?;
    assert_eq!(dashboards.len(), 1);
    assert_eq!(dashboards[0].uid, "dash-1");
    assert_eq!(dashboards[0].name(), Some("Latency"));
    mock.assert_hits_async(1).await;
    Ok(())
}

#[tokio::test]
async fn get_dashboard() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/v2/dashboards/uid/dash-1");
        then.status(200).json_body(json!({
            "uid": "dash-1",
            "version": 5,
            "dashboard": { "name": "Errors" }
        }));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let dashboard = client.dashboards().get("dash-1").await?;
    assert_eq!(dashboard.version, Some(5));
    mock.assert_hits_async(1).await;
    Ok(())
}

#[tokio::test]
async fn create_dashboard() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/v2/dashboards");
        then.status(200).json_body(json!({
            "status": "created",
            "dashboard": {
                "uid": "dash-2",
                "version": 1,
                "dashboard": { "name": "New" }
            }
        }));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let doc = DashboardDocument {
        name: Some("New".into()),
        ..Default::default()
    };
    let res = client.dashboards().create(&doc, None).await?;
    assert_eq!(res.status, DashboardWriteStatus::Created);
    assert_eq!(res.dashboard.uid, "dash-2");
    mock.assert_hits_async(1).await;
    Ok(())
}

#[tokio::test]
async fn put_dashboard_with_default_options() -> Result<(), Box<dyn std::error::Error>> {
    // `put(uid, &doc, None)` must work: the `Into<Option<UpsertOptions>>`
    // shortcut should produce the same wire shape as
    // `UpsertOptions::default()`.
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(PUT).path("/v2/dashboards/uid/dash-1");
        then.status(200).json_body(json!({
            "status": "updated",
            "dashboard": {
                "uid": "dash-1",
                "version": 4,
                "dashboard": { "name": "x" }
            }
        }));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let doc = DashboardDocument::default();
    let res = client.dashboards().put("dash-1", &doc, None).await?;
    assert_eq!(res.status, DashboardWriteStatus::Updated);
    assert_eq!(res.dashboard.version, Some(4));
    mock.assert_hits_async(1).await;
    Ok(())
}

#[tokio::test]
async fn put_dashboard_version_conflict() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(PUT).path("/v2/dashboards/uid/dash-1");
        then.status(412).json_body(json!({
            "code": "version_mismatch",
            "message": "stale version",
            "currentVersion": 7,
            "uid": "dash-1"
        }));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let doc = DashboardDocument::default();
    let err = client
        .dashboards()
        .put(
            "dash-1",
            &doc,
            UpsertOptions {
                expected_version: Some(3),
                ..Default::default()
            },
        )
        .await
        .expect_err("expected DashboardVersionConflict");
    match err {
        Error::DashboardVersionConflict { uid, current } => {
            assert_eq!(uid, "dash-1");
            assert_eq!(current, 7);
        }
        other => panic!("expected DashboardVersionConflict, got {:?}", other),
    }
    mock.assert_hits_async(1).await;
    Ok(())
}

#[tokio::test]
async fn put_dashboard_rate_limited() -> Result<(), Box<dyn std::error::Error>> {
    // Regression test for the C2 refactor: dashboard writes used to
    // bypass `check_error`, silently dropping limit-mapping. Now that
    // we go through the shared path, a 429 with rate-limit headers must
    // surface as `Error::RateLimitExceeded`.
    let server = MockServer::start();
    let reset = Utc::now() + Duration::seconds(60);
    let mock = server.mock(|when, then| {
        when.method(PUT).path("/v2/dashboards/uid/dash-1");
        then.status(429)
            .json_body(json!({ "message": "rate limit exceeded" }))
            .header(limits::HEADER_RATE_SCOPE, "user")
            .header(limits::HEADER_RATE_LIMIT, "42")
            .header(limits::HEADER_RATE_REMAINING, "0")
            .header(limits::HEADER_RATE_RESET, format!("{}", reset.timestamp()));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let doc = DashboardDocument::default();
    let err = client
        .dashboards()
        .put(
            "dash-1",
            &doc,
            UpsertOptions {
                expected_version: Some(3),
                ..Default::default()
            },
        )
        .await
        .expect_err("expected rate limit error");
    match err {
        Error::RateLimitExceeded { scope, limits } => {
            assert_eq!(scope, "user");
            assert_eq!(limits.limit, 42);
            assert_eq!(limits.remaining, 0);
        }
        other => panic!("expected RateLimitExceeded, got {:?}", other),
    }
    mock.assert_hits_async(1).await;
    Ok(())
}

#[tokio::test]
async fn put_dashboard_412_without_current_version_falls_back_to_axiom(
) -> Result<(), Box<dyn std::error::Error>> {
    // A 412 without `currentVersion` shouldn't panic on decode; we fall
    // back to the generic `Error::Axiom` envelope.
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(PUT).path("/v2/dashboards/uid/dash-1");
        then.status(412)
            .json_body(json!({ "message": "precondition failed" }));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    let doc = DashboardDocument::default();
    let err = client
        .dashboards()
        .put(
            "dash-1",
            &doc,
            UpsertOptions {
                expected_version: Some(3),
                ..Default::default()
            },
        )
        .await
        .expect_err("expected generic Axiom error");
    match err {
        Error::Axiom(axiom) => {
            assert_eq!(axiom.status, 412);
            assert_eq!(axiom.message.as_deref(), Some("precondition failed"));
            // The fallback must preserve method/path context so the error
            // is debuggable, not the synthetic `“unknown PUT ”` we had
            // before threading `Response::method`/`path` through.
            assert_eq!(axiom.method, ::http::Method::PUT);
            assert_eq!(axiom.path, "/v2/dashboards/uid/dash-1");
        }
        other => panic!("expected Error::Axiom, got {:?}", other),
    }
    mock.assert_hits_async(1).await;
    Ok(())
}

#[tokio::test]
async fn delete_dashboard() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(DELETE).path("/v2/dashboards/uid/dash-1");
        then.status(204);
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_token("xaat-nope")
        .build()?;

    client.dashboards().delete("dash-1").await?;
    mock.assert_hits_async(1).await;
    Ok(())
}
