mod camera;
mod config;
mod health;
mod metrics;
mod storage;

use anyhow::{Context, Result};
use futures::stream::{FuturesUnordered, StreamExt};
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook_tokio::Signals;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("camera_recorder=info".parse().unwrap()),
        )
        .json()
        .init();

    info!("Starting camera recorder service");

    // Initialize metrics
    metrics::init_metrics().context("Failed to initialize metrics")?;

    // Load configuration
    let config = if let Ok(config_path) = std::env::var("CONFIG_PATH") {
        config::Config::from_file(&config_path)?
    } else {
        info!("Loading config from environment variables");
        config::Config::from_env()?
    };

    info!("Loaded configuration for {} cameras", config.cameras.len());

    // Create shared state
    let state = Arc::new(RwLock::new(ServiceState {
        cameras_connected: 0,
        total_cameras: config.cameras.len(),
    }));

    // Initialize S3 client for SeaweedFS
    let s3_client = storage::S3Client::new(&config.storage).await?;
    info!("Connected to SeaweedFS at {}", config.storage.endpoint);

    // Ensure bucket exists
    s3_client
        .ensure_bucket_exists()
        .await
        .context("Failed to create/verify bucket")?;

    // Create upload channel
    let (upload_tx, upload_rx) = mpsc::channel(1000);

    // Start upload worker
    let upload_worker = storage::UploadWorker::new(
        upload_rx,
        s3_client.clone(),
        config.upload.max_concurrent,
        config.upload.max_retries,
        config.upload.retry_backoff_secs,
    );

    let upload_handle = tokio::spawn(async move {
        upload_worker.run().await;
    });

    info!("Upload worker started");

    // Start metrics server
    let metrics_state = state.clone();
    let metrics_port = config.service.metrics_port;
    tokio::spawn(async move {
        if let Err(e) = metrics::server::start_server(metrics_port, metrics_state).await {
            error!("Metrics server failed: {}", e);
        }
    });

    // Start camera recorders
    let mut recorder_tasks = FuturesUnordered::new();

    for camera_config in &config.cameras {
        let camera_cfg = camera_config.clone();
        let recording_cfg = config.recording.clone();
        let upload_cfg = config.upload.clone();
        let upload_tx_clone = upload_tx.clone();
        let state_clone = state.clone();

        // Initialize metrics for this camera
        metrics::CAMERA_CONNECTED
            .with_label_values(&[&camera_cfg.id])
            .set(0.0);

        let task = tokio::spawn(async move {
            if let Err(e) = camera::recorder::run_recorder(
                camera_cfg,
                recording_cfg,
                upload_cfg,
                upload_tx_clone,
                state_clone,
            )
            .await
            {
                error!("Camera recorder failed: {}", e);
            }
        });

        recorder_tasks.push(task);
    }

    info!("All camera recorders started");

    // Handle shutdown signals
    let mut signals = Signals::new([SIGTERM, SIGINT])?;

    tokio::select! {
        _ = async {
            if let Some(SIGTERM | SIGINT) = signals.next().await {
                info!("Received shutdown signal, stopping gracefully...");
            }
        } => {
            info!("Initiating graceful shutdown");
        },
        _ = async {
            while let Some(result) = recorder_tasks.next().await {
                if let Err(e) = result {
                    error!("Recorder task panicked: {}", e);
                }
            }
        } => {
            warn!("All recorder tasks completed unexpectedly");
        }
    }

    // Drop upload sender to signal upload worker to finish
    drop(upload_tx);

    // Wait for upload worker to finish pending uploads
    info!("Waiting for pending uploads to complete...");
    let _ = tokio::time::timeout(tokio::time::Duration::from_secs(60), upload_handle).await;

    info!("Camera recorder service stopped");
    Ok(())
}

/// Shared service state
#[derive(Debug)]
pub struct ServiceState {
    pub cameras_connected: usize,
    pub total_cameras: usize,
}
