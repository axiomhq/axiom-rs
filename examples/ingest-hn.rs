use std::env;

use axiom_rs::Client;
use futures::stream::{self, StreamExt};
use serde_json::Value as JsonValue;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

const BASE_URL: &str = "https://hacker-news.firebaseio.com";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fmt_layer = tracing_subscriber::fmt::layer().pretty();
    let filter_layer = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?;
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .try_init()?;

    let dataset_name = env::var("DATASET_NAME").expect("Missing DATASET_NAME");
    let max_item_id = reqwest::get(format!("{}/v0/maxitem.json", BASE_URL))
        .await?
        .text()
        .await?
        .parse::<u64>()?;
    let stream = stream::iter(0..max_item_id)
        .map(|id| async move {
            info!(?id, "Fetching item");
            reqwest::get(format!("{}/v0/item/{}.json", BASE_URL, id))
                .await?
                .json::<JsonValue>()
                .await
        })
        .buffered(100);
    Client::new()?
        .try_ingest_stream(dataset_name, stream)
        .await?;
    Ok(())
}
