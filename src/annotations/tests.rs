use super::{requests, Annotation};
use crate::Client;
use chrono::DateTime;
use httpmock::prelude::*;
use serde_json::json;

#[tokio::test]
async fn get() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let server_reply = Annotation {
        id: "42".to_string(),
        annotation_type: "cake".to_string(),
        datasets: vec!["snot".to_string(), "snot".to_string()],
        description: None,
        title: Some("cookie".to_string()),
        url: None,
        time: DateTime::parse_from_rfc3339("2024-02-06T11:39:28.382Z")
            .expect("we know the time is right"),
        end_time: None,
    };
    let mock = server.mock(|when, then| {
        when.method(GET).path("/v2/annotations/42");
        then.status(200).json_body(json!(server_reply.clone()));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_token("xapt-nope")
        .build()?;

    let r = client.annotations().get("42").await?;
    assert_eq!(r, server_reply);
    mock.assert_hits_async(1).await;

    Ok(())
}

#[tokio::test]
async fn lsit() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let server_reply = Annotation {
        id: "42".to_string(),
        annotation_type: "cake".to_string(),
        datasets: vec!["snot".to_string(), "snot".to_string()],
        description: None,
        title: Some("cookie".to_string()),
        url: None,
        time: DateTime::parse_from_rfc3339("2024-02-06T11:39:28.382Z")
            .expect("we know the time is right"),
        end_time: None,
    };
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v2/annotations")
            .query_param("start", "2024-02-06T11:39:28.382Z");
        then.status(200)
            .json_body(json!(vec![server_reply.clone(), server_reply.clone()]));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_token("xapt-nope")
        .build()?;

    let req = requests::List::builder()
        .with_start(
            DateTime::parse_from_rfc3339("2024-02-06T11:39:28.382Z")
                .expect("we know the time is right"),
        )?
        .build();
    let r = client.annotations().list(req).await?;
    assert_eq!(r, vec![server_reply.clone(), server_reply]);
    mock.assert_hits_async(1).await;
    Ok(())
}

#[tokio::test]
async fn delete() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(DELETE).path("/v2/annotations/42");
        then.status(204);
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_token("xapt-nope")
        .build()?;

    client.annotations().delete("42").await?;
    mock.assert_hits_async(1).await;

    Ok(())
}

#[tokio::test]
async fn create() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start();
    let server_reply = Annotation {
        id: "42".to_string(),
        annotation_type: "cake".to_string(),
        datasets: vec!["snot".to_string(), "snot".to_string()],
        description: None,
        title: Some("cookie".to_string()),
        url: None,
        time: DateTime::parse_from_rfc3339("2024-02-06T11:39:28.382Z")
            .expect("we know the time is right"),
        end_time: None,
    };
    let mock = server.mock(|when, then| {
        when.method(POST).path("/v2/annotations").json_body_obj(
            &requests::Create::builder()
                .with_type("cake")
                .expect("known ok")
                .with_datasets(vec!["snot".to_string(), "snot".to_string()])
                .expect("known ok")
                .with_title("cookie")
                .build(),
        );
        then.status(200).json_body(json!(server_reply));
    });
    let client = Client::builder()
        .no_env()
        .with_url(server.base_url())
        .with_token("xapt-nope")
        .build()?;

    let req = requests::Create::builder()
        .with_type("cake")
        .expect("known ok")
        .with_datasets(vec!["snot".to_string(), "snot".to_string()])
        .expect("known ok")
        .with_title("cookie")
        .build();
    let r = client.annotations().create(req).await?;
    assert_eq!(r, server_reply);
    mock.assert_hits_async(1).await;
    Ok(())
}
