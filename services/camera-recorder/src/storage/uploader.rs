use super::S3Client;
use crate::metrics;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use tokio::time::{sleep, Duration, Instant};
use tracing::{error, info, warn};

/// Information about a completed segment ready for upload
#[derive(Debug, Clone)]
pub struct SegmentInfo {
    pub camera_id: String,
    pub local_path: PathBuf,
    pub timestamp: DateTime<Utc>,
}

pub struct UploadWorker {
    rx: mpsc::Receiver<SegmentInfo>,
    s3_client: S3Client,
    max_retries: u32,
    retry_backoff_secs: u64,
    semaphore: Arc<Semaphore>,
}

impl UploadWorker {
    pub fn new(
        rx: mpsc::Receiver<SegmentInfo>,
        s3_client: S3Client,
        max_concurrent: usize,
        max_retries: u32,
        retry_backoff_secs: u64,
    ) -> Self {
        Self {
            rx,
            s3_client,
            max_retries,
            retry_backoff_secs,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Run the upload worker (processes segments from queue)
    pub async fn run(mut self) {
        info!("Upload worker started");

        while let Some(segment) = self.rx.recv().await {
            let s3_client = self.s3_client.clone();
            let semaphore = self.semaphore.clone();
            let max_retries = self.max_retries;
            let retry_backoff_secs = self.retry_backoff_secs;

            // Spawn upload task (limited by semaphore)
            tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();

                if let Err(e) =
                    upload_segment_with_retry(segment, s3_client, max_retries, retry_backoff_secs)
                        .await
                {
                    error!(error = %e, "Failed to upload segment after retries");
                }
            });
        }

        info!("Upload worker stopped (channel closed)");
    }
}

/// Upload a segment with retry logic
async fn upload_segment_with_retry(
    segment: SegmentInfo,
    s3_client: S3Client,
    max_retries: u32,
    retry_backoff_secs: u64,
) -> Result<()> {
    let filename = segment
        .local_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("unknown");

    // Build S3 key: camera-{id}/{YYYYMMDD}/{filename}
    let date_str = segment.timestamp.format("%Y%m%d").to_string();
    let s3_key = format!("{}/{}/{}", segment.camera_id, date_str, filename);

    info!(
        camera_id = %segment.camera_id,
        segment = %filename,
        s3_key = %s3_key,
        "Starting upload"
    );

    let mut retry = 0;
    let start_time = Instant::now();

    loop {
        match s3_client.upload_file(&segment.local_path, &s3_key).await {
            Ok(()) => {
                let duration = start_time.elapsed().as_secs_f64();

                info!(
                    camera_id = %segment.camera_id,
                    segment = %filename,
                    s3_key = %s3_key,
                    duration_secs = duration,
                    "Upload successful"
                );

                // Update metrics
                metrics::SEGMENTS_UPLOADED
                    .with_label_values(&[&segment.camera_id])
                    .inc();
                metrics::UPLOAD_DURATION
                    .with_label_values(&[&segment.camera_id])
                    .observe(duration);

                // Cleanup local file
                if let Err(e) = s3_client.cleanup_local_file(&segment.local_path).await {
                    warn!(
                        error = %e,
                        path = %segment.local_path.display(),
                        "Failed to cleanup local file"
                    );
                }

                return Ok(());
            }
            Err(e) => {
                retry += 1;

                // Update failure metrics
                metrics::UPLOAD_FAILURES
                    .with_label_values(&[&segment.camera_id])
                    .inc();

                if retry >= max_retries {
                    error!(
                        error = %e,
                        camera_id = %segment.camera_id,
                        segment = %filename,
                        retries = retry,
                        "Upload failed after max retries"
                    );
                    return Err(e);
                }

                let wait_secs = retry_backoff_secs * 2u64.pow(retry - 1);
                warn!(
                    error = %e,
                    camera_id = %segment.camera_id,
                    segment = %filename,
                    retry = retry,
                    wait_secs = wait_secs,
                    "Upload failed, retrying"
                );
                sleep(Duration::from_secs(wait_secs)).await;
            }
        }
    }
}
