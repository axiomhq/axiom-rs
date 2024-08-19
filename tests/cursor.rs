#![cfg(feature = "integration-tests")]
use axiom_rs::{datasets::*, Client};
use chrono::{Duration, Utc};
use serde_json::json;
use std::env;
use test_context::{test_context, AsyncTestContext};

struct Context {
    client: Client,
    dataset: Dataset,
}

impl AsyncTestContext for Context {
    async fn setup() -> Self {
        let client = Client::new().unwrap();

        let dataset_name = format!(
            "test-axiom-rs-{}",
            env::var("AXIOM_DATASET_SUFFIX").expect("AXIOM_DATASET_SUFFIX is not set"),
        );

        // Delete dataset in case we have a zombie
        client.datasets().delete(&dataset_name).await.ok();

        let dataset = client
            .datasets()
            .create(&dataset_name, "bar")
            .await
            .unwrap();
        assert_eq!(dataset_name.clone(), dataset.name);
        assert_eq!("bar".to_string(), dataset.description);

        Context { client, dataset }
    }

    async fn teardown(self) {
        self.client.datasets().delete(self.dataset.name).await.ok();
    }
}

#[cfg(feature = "tokio")]
#[test_context(Context)]
#[tokio::test]
async fn test_cursor(ctx: &mut Context) {
    test_cursor_impl(ctx).await;
}

#[cfg(feature = "async-std")]
#[test_context(Context)]
#[async_std::test]
async fn test_cursor(ctx: &mut Context) {
    test_cursor_impl(ctx).await;
}

async fn test_cursor_impl(ctx: &mut Context) {
    // Let's update the dataset.
    let dataset = ctx
        .client
        .datasets()
        .update(
            &ctx.dataset.name,
            "This is a soon to be filled test dataset",
        )
        .await
        .unwrap();
    ctx.dataset = dataset;

    // Get the dataset and make sure it matches what we have updated it to.
    let dataset = ctx.client.datasets().get(&ctx.dataset.name).await.unwrap();
    assert_eq!(ctx.dataset.name, dataset.name);
    assert_eq!(ctx.dataset.name, dataset.name);
    assert_eq!(ctx.dataset.description, dataset.description);

    // List all datasets and make sure the created dataset is part of that
    // list.
    let datasets = ctx.client.datasets().list().await.unwrap();
    datasets
        .iter()
        .find(|dataset| dataset.name == ctx.dataset.name)
        .expect("Expected dataset to be in the list");

    let mut events = Vec::new();

    // iterate 1000 times
    let event_time = Utc::now();
    for _ in 0..1000 {
        events.push(json!({
            "_time": event_time,
            "remote_ip": "93.180.71.2",
            "remote_user": "-",
            "request": "GET /downloads/product_1 HTTP/1.1",
            "response": 304,
            "bytes": 0,
            "referrer": "-",
            "agent": "Debian APT-HTTP/1.3 (0.8.16~exp12ubuntu10.21)"
        }));
    }

    let ingest_status = ctx.client.ingest(&ctx.dataset.name, &events).await.unwrap();
    assert_eq!(ingest_status.ingested, 1000);
    assert_eq!(ingest_status.failed, 0);
    assert_eq!(ingest_status.failures.len(), 0);

    let start_time = Utc::now() - Duration::minutes(1);
    let end_time = Utc::now() + Duration::minutes(1);

    let apl_query_result = ctx
        .client
        .query(
            &format!("['{}'] | sort by _time desc", ctx.dataset.name),
            QueryOptions {
                start_time: Some(start_time),
                end_time: Some(end_time),
                save: true,
                include_cursor_field: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(apl_query_result.saved_query_id.is_some());
    assert_eq!(1000, apl_query_result.tables[0].len());

    let table = &apl_query_result.tables[0];

    let row = table.get_row(500).unwrap();

    let mid_row_id = &row.get_field("_cursor").expect("column _cursor not found");

    let apl_query_result = ctx
        .client
        .query(
            &format!("['{}'] | sort by _time desc", ctx.dataset.name),
            QueryOptions {
                start_time: Some(start_time),
                end_time: Some(end_time),
                include_cursor: true,
                cursor: Some(mid_row_id.to_string()),
                save: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(apl_query_result.saved_query_id.is_some());
    assert_eq!(500, apl_query_result.tables.len());
    assert_eq!(500, apl_query_result.tables[0].len());
}
