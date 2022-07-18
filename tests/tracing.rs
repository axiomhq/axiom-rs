use tracing::event;
use tracing::Level;
use axiom_rs::tracing::TelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::test]
async fn test_tracing_layer() {
    tracing_subscriber::registry().with(TelemetryLayer).init();

    event!(Level::INFO, "Tracing layer initialized successfully")
    // TODO: check whats sent through axiom client
}