use crate::metrics::REGISTRY;
use crate::ServiceState;
use anyhow::Result;
use axum::{extract::State as AxumState, routing::get, Router};
use prometheus::Encoder;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub async fn start_server(port: u16, state: Arc<RwLock<ServiceState>>) -> Result<()> {
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    info!("Starting metrics server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_handler() -> &'static str {
    "OK"
}

async fn ready_handler(AxumState(state): AxumState<Arc<RwLock<ServiceState>>>) -> String {
    let s = state.read().await;
    if s.cameras_connected == s.total_cameras {
        "READY".to_string()
    } else {
        format!(
            "NOT_READY: {}/{} cameras connected",
            s.cameras_connected, s.total_cameras
        )
    }
}

async fn metrics_handler() -> String {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = REGISTRY.gather();

    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    String::from_utf8(buffer).unwrap_or_else(|_| String::from("# Encoding error\n"))
}
