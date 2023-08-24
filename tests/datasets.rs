use axiom_rs::{datasets::*, Client};
use chrono::{Duration, Utc};
#[cfg(not(feature = "blocking"))]
use futures::StreamExt;
use maybe_async::maybe_async;
use serde_json::json;
#[cfg(feature = "blocking")]
use std::thread::sleep;
use std::{env, time::Duration as StdDuration};
#[cfg(not(feature = "blocking"))]
use tokio::time::sleep;

#[maybe_async]
#[cfg_attr(feature = "blocking", test)]
#[cfg_attr(feature = "tokio", tokio::test)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn test_datasets_impl() {
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

    // Let's ingest some data
    const PAYLOAD: &'static str = r#"[
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
    let ingest_status = client
        .ingest_bytes(
            &dataset.name,
            PAYLOAD,
            ContentType::Json,
            ContentEncoding::Identity,
        )
        .await
        .unwrap();
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
    let ingest_status = client.ingest(&dataset.name, &events).await.unwrap();
    assert_eq!(ingest_status.ingested, 2);
    assert_eq!(ingest_status.failed, 0);
    assert_eq!(ingest_status.failures.len(), 0);

    #[cfg(not(feature = "blocking"))]
    {
        // ... a small stream
        let stream = futures_util::stream::iter(events.clone());
        let ingest_status = client.ingest_stream(&dataset.name, stream).await.unwrap();
        assert_eq!(ingest_status.ingested, 2);
        assert_eq!(ingest_status.failed, 0);
        assert_eq!(ingest_status.failures.len(), 0);

        // ... and a big stream (4321 items)
        let stream = futures_util::stream::iter(events).cycle().take(4321);
        let ingest_status = client.ingest_stream(&dataset.name, stream).await.unwrap();
        assert_eq!(ingest_status.ingested, 4321);
        assert_eq!(ingest_status.failed, 0);
        assert_eq!(ingest_status.failures.len(), 0);
    }

    // Give the db some time to write the data.
    sleep(StdDuration::from_secs(15)).await;

    let expected_event_count = if cfg!(feature = "blocking") {
        4
    } else {
        4327 // From async stream tests
    };

    // Get the dataset info and make sure four events have been ingested.
    let info = client.datasets.info(&dataset.name).await.unwrap();
    assert_eq!(dataset.name, info.stat.name);
    assert_eq!(expected_event_count, info.stat.num_events);
    assert!(info.fields.len() > 0);

    // Run a query and make sure we see some results.
    #[allow(deprecated)]
    let simple_query_result = client
        .query_legacy(
            &dataset.name,
            LegacyQuery {
                start_time: Some(Utc::now() - Duration::minutes(1)),
                end_time: Some(Utc::now()),
                ..Default::default()
            },
            Some(LegacyQueryOptions {
                save_as_kind: QueryKind::Analytics,
                ..Default::default()
            }),
        )
        .await
        .unwrap();
    assert!(simple_query_result.saved_query_id.is_some());
    // assert_eq!(1, simple_query_result.status.blocks_examined);
    assert_eq!(
        expected_event_count,
        simple_query_result.status.rows_examined
    );
    assert_eq!(
        expected_event_count,
        simple_query_result.status.rows_matched
    );
    if cfg!(feature = "blocking") {
        assert_eq!(4, simple_query_result.matches.len());
    } else {
        assert_eq!(1000, simple_query_result.matches.len());
    }

    // Run another query but using APL.
    let apl_query_result = client
        .query(
            format!("['{}']", dataset.name),
            QueryOptions {
                save: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(apl_query_result.saved_query_id.is_some());
    // assert_eq!(1, apl_query_result.status.blocks_examined);
    assert_eq!(expected_event_count, apl_query_result.status.rows_examined);
    assert_eq!(expected_event_count, apl_query_result.status.rows_matched);
    if cfg!(feature = "blocking") {
        assert_eq!(4, apl_query_result.matches.len());
    } else {
        assert_eq!(1000, apl_query_result.matches.len());
    }

    // Run a more complex query.
    let query = LegacyQuery {
        start_time: Some(Utc::now() - Duration::minutes(1)),
        end_time: Some(Utc::now()),
        aggregations: vec![Aggregation {
            alias: Some("event_count".to_string()),
            op: AggregationOp::Count,
            field: "*".to_string(),
            argument: None,
        }],
        group_by: vec!["success".to_string(), "remote_ip".to_string()],
        filter: Some(Filter {
            op: FilterOp::Equal,
            field: "response".to_string(),
            value: json!(304),
            ..Default::default()
        }),
        order: vec![
            Order {
                field: "success".to_string(),
                desc: true,
            },
            Order {
                field: "remote_ip".to_string(),
                desc: false,
            },
        ],
        virtual_fields: vec![VirtualField {
            alias: "success".to_string(),
            expr: "response < 400".to_string(),
        }],
        projections: vec![Projection {
            field: "remote_ip".to_string(),
            alias: Some("ip".to_string()),
        }],
        ..Default::default()
    };
    #[allow(deprecated)]
    let query_result = client
        .query_legacy(
            &dataset.name,
            query,
            LegacyQueryOptions {
                save_as_kind: QueryKind::Analytics,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(expected_event_count, query_result.status.rows_examined);
    assert_eq!(expected_event_count, query_result.status.rows_matched);
    assert!(query_result.buckets.totals.len() == 2);
    let agg = query_result
        .buckets
        .totals
        .get(0)
        .unwrap()
        .aggregations
        .get(0)
        .unwrap();
    assert_eq!("event_count", agg.alias);
    if cfg!(feature = "blocking") {
        assert_eq!(2, agg.value);
    } else {
        assert_eq!(2164, agg.value);
    }

    // Trim the dataset down to a minimum.
    client
        .datasets
        .trim(&dataset.name, Duration::seconds(1))
        .await
        .unwrap();
}
