#![cfg(test)]

use crate::{
    dashboards::model::{DashboardDocument, DashboardWriteStatus},
    error::Error,
    Client,
};
use httpmock::prelude::*;
use serde_json::json;

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
    mock.assert();
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
    mock.assert();
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

    let mut doc = DashboardDocument::default();
    doc.name = Some("New".into());
    let res = client.dashboards().create(&doc, None, None).await?;
    assert_eq!(res.status, DashboardWriteStatus::Created);
    assert_eq!(res.dashboard.uid, "dash-2");
    mock.assert();
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
        .put("dash-1", &doc, Some(3), false, None)
        .await
        .unwrap_err();
    match err {
        Error::DashboardVersionConflict { uid, current } => {
            assert_eq!(uid, "dash-1");
            assert_eq!(current, 7);
        }
        other => panic!("expected DashboardVersionConflict, got {:?}", other),
    }
    mock.assert();
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
    mock.assert();
    Ok(())
}
