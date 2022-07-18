use tracing::{Subscriber, Event};
use tracing::field::Field;
use tracing_subscriber::Layer;
use std::collections::{BTreeMap};

pub struct TelemetryLayer;
struct JsonVisitor<'a>(&'a mut BTreeMap<String, serde_json::Value>);

impl<S> Layer<S> for TelemetryLayer where S: Subscriber{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut fields: BTreeMap<String, serde_json::Value> = BTreeMap::new();
        let mut visitor = JsonVisitor(&mut fields);
        event.record(&mut visitor);


        // Output the event in JSON
        let payload = serde_json::json!({
        // "target": event.metadata().target(),
        "level": format!("{:?}", event.metadata().level()),
        "fields": fields,
    });
        for field in event.fields() {
            if field.name() == "message" {
                println!("{}", field.name());
            }
        }
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
        // TODO: send payload to axiom
    }
}

impl<'a> tracing::field::Visit for JsonVisitor<'a> {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.0.insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.0.insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0.insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.0.insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        // self.0.insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        // self.0.insert(field.name().to_string(), serde_json::json!(value));
    }
}

