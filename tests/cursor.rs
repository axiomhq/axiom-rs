use axiom_rs::{datasets::*, Client};
use chrono::{Duration, Utc};
use maybe_async::maybe_async;
use serde_json::json;
use std::env;

#[maybe_async]
#[cfg_attr(feature = "blocking", test)]
#[cfg_attr(feature = "tokio", tokio::test)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn test_cursor() {
    let client = Client::new().unwrap();

    let dataset_name = format!(
        "test-axiom-rs-{}",
        env::var("AXIOM_DATASET_SUFFIX").expect("AXIOM_DATASET_SUFFIX is not set"),
    );

    // Delete dataset in case we have a zombie
    client.datasets.delete(&dataset_name).await.ok();

    let dataset = client.datasets.create(&dataset_name, "bar").await.unwrap();
    assert_eq!(dataset_name.clone(), dataset.name);
    assert_eq!("bar".to_string(), dataset.description);

    // Let's update the dataset.
    let dataset = client
        .datasets
        .update(&dataset.name, "This is a soon to be filled test dataset")
        .await
        .unwrap();

    // Get the dataset and make sure it matches what we have updated it to.
    let dataset = client.datasets.get(&dataset.name).await.unwrap();
    assert_eq!(dataset.name, dataset.name);
    assert_eq!(dataset.name, dataset.name);
    assert_eq!(dataset.description, dataset.description);

    // List all datasets and make sure the created dataset is part of that
    // list.
    let datasets = client.datasets.list().await.unwrap();
    datasets
        .iter()
        .find(|dataset| dataset.name == dataset.name)
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

    let ingest_status = client.ingest(&dataset.name, &events).await.unwrap();
    assert_eq!(ingest_status.ingested, 1000);
    assert_eq!(ingest_status.failed, 0);
    assert_eq!(ingest_status.failures.len(), 0);

    let start_time = Utc::now() - Duration::minutes(1);
    let end_time = Utc::now() + Duration::minutes(1);

    let apl_query_result = client
        .query(
            format!("['{}'] | sort by _time desc", dataset.name),
            QueryOptions {
                start_time: Some(start_time),
                end_time: Some(end_time),
                save: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(apl_query_result.saved_query_id.is_some());
    assert_eq!(1000, apl_query_result.matches.len());

    let mid_row_id = &apl_query_result.matches[500].row_id;

    let apl_query_result = client
        .query(
            format!("['{}'] | sort by _time desc", dataset.name),
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
    assert_eq!(500, apl_query_result.matches.len());
}
