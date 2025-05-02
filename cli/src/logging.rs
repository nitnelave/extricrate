use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::default());
    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_forest::ForestLayer::default())
        .init();
}
