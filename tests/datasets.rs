#![cfg(feature = "integration-tests")]
use axiom_rs::{datasets::*, Client};
use futures::StreamExt;
use serde_json::json;
use std::{env, time::Duration as StdDuration};
use test_context::{test_context, AsyncTestContext};

struct Context {
    client: Client,
    dataset: Dataset,
}

impl AsyncTestContext for Context {
    async fn setup() -> Context {
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
async fn test_datasets(ctx: &mut Context) -> Result<(), Box<dyn std::error::Error>> {
    test_datasets_impl(ctx).await
}
#[cfg(feature = "async-std")]
#[test_context(Context)]
#[async_std::test]
async fn test_datasets(ctx: &mut Context) -> Result<(), Box<dyn std::error::Error>> {
    Ok(test_datasets_impl(ctx).await?)
}

async fn test_datasets_impl(ctx: &mut Context) -> Result<(), Box<dyn std::error::Error>> {
    // Let's update the dataset.
    let dataset = ctx
        .client
        .datasets()
        .update(
            &ctx.dataset.name,
            "This is a soon to be filled test dataset",
        )
        .await?;
    ctx.dataset = dataset;

    // Get the dataset and make sure it matches what we have updated it to.
    let dataset = ctx.client.datasets().get(&ctx.dataset.name).await?;
    assert_eq!(ctx.dataset.name, dataset.name);
    assert_eq!(ctx.dataset.name, dataset.name);
    assert_eq!(ctx.dataset.description, dataset.description);

    // List all datasets and make sure the created dataset is part of that
    // list.
    let datasets = ctx.client.datasets().list().await?;
    datasets
        .iter()
        .find(|dataset| dataset.name == ctx.dataset.name)
        .expect("Expected dataset to be in the list");

    // Let's ingest some data
    const PAYLOAD: &str = r#"[
	{
		"time": "17/May/2015:08:05:30 +0000",
		"remote_ip": "93.180.71.1",
		"remote_user": "-",
		"request": "GET /downloads/product_1 HTTP/1.1",
		"response": 304,
		"bytes": 0,
		"referrer": "-",
		"agent": "Debian APT-HTTP/1.3 (0.8.16~exp12ubuntu10.21)"
	},
	{
		"time": "17/May/2015:08:05:31 +0000",
		"remote_ip": "93.180.71.2",
		"remote_user": "-",
		"request": "GET /downloads/product_1 HTTP/1.1",
		"response": 304,
		"bytes": 0,
		"referrer": "-",
		"agent": "Debian APT-HTTP/1.3 (0.8.16~exp12ubuntu10.21)"
	}
]"#;
    let ingest_status = ctx
        .client
        .ingest_bytes(
            &ctx.dataset.name,
            PAYLOAD,
            ContentType::Json,
            ContentEncoding::Identity,
        )
        .await?;
    assert_eq!(ingest_status.ingested, 2);
    assert_eq!(ingest_status.failed, 0);
    assert_eq!(ingest_status.failures.len(), 0);
    assert_eq!(ingest_status.processed_bytes, PAYLOAD.len() as u64);

    // ... and a map.
    let events = vec![
        json!({
            "time": "17/May/2015:08:05:30 +0000",
            "remote_ip": "93.180.71.1",
            "remote_user": "-",
            "request": "GET /downloads/product_1 HTTP/1.1",
            "response": 304,
            "bytes": 0,
            "referrer": "-",
            "agent": "Debian APT-HTTP/1.3 (0.8.16~exp12ubuntu10.21)",
        }),
        json!({
            "time": "17/May/2015:08:05:31 +0000",
            "remote_ip": "93.180.71.2",
            "remote_user": "-",
            "request": "GET /downloads/product_1 HTTP/1.1",
            "response": 304,
            "bytes": 0,
            "referrer": "-",
            "agent": "Debian APT-HTTP/1.3 (0.8.16~exp12ubuntu10.21)"
        }),
    ];
    let ingest_status = ctx.client.ingest(&ctx.dataset.name, &events).await?;
    assert_eq!(ingest_status.ingested, 2);
    assert_eq!(ingest_status.failed, 0);
    assert_eq!(ingest_status.failures.len(), 0);

    // ... a small stream
    let stream = futures_util::stream::iter(events.clone());
    let ingest_status = ctx.client.ingest_stream(&ctx.dataset.name, stream).await?;
    assert_eq!(ingest_status.ingested, 2);
    assert_eq!(ingest_status.failed, 0);
    assert_eq!(ingest_status.failures.len(), 0);

    // ... and a big stream (4321 items)
    let stream = futures_util::stream::iter(events).cycle().take(4321);
    let ingest_status = ctx.client.ingest_stream(&ctx.dataset.name, stream).await?;
    assert_eq!(ingest_status.ingested, 4321);
    assert_eq!(ingest_status.failed, 0);
    assert_eq!(ingest_status.failures.len(), 0);

    // Give the db some time to write the data.
    tokio::time::sleep(StdDuration::from_secs(15)).await;

    // Get the dataset info and make sure four events have been ingested.
    let info = ctx.client.datasets().info(&ctx.dataset.name).await?;
    assert_eq!(ctx.dataset.name, info.stat.name);
    assert_eq!(4327, info.stat.num_events);
    assert!(!info.fields.is_empty());

    // Run another query but using APL.
    let apl_query_result = ctx
        .client
        .query(
            &format!("['{}']", ctx.dataset.name),
            QueryOptions {
                save: true,
                ..Default::default()
            },
        )
        .await?;
    assert!(apl_query_result.saved_query_id.is_some());
    // assert_eq!(1, apl_query_result.status.blocks_examined);
    assert_eq!(4327, apl_query_result.status.rows_examined);
    assert_eq!(4327, apl_query_result.status.rows_matched);
    assert_eq!(2, apl_query_result.tables.len());
    assert_eq!(1000, apl_query_result.tables[0].len());

    Ok(())
}
