use crate::config::{CameraConfig, RecordingConfig, UploadConfig};
use crate::metrics;
use crate::storage::SegmentInfo;
use crate::ServiceState;
use anyhow::{Context, Result};
use chrono::Utc;

use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{mpsc, RwLock};
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

/// Run recorder for a single camera
pub async fn run_recorder(
    camera: CameraConfig,
    recording: RecordingConfig,
    _upload: UploadConfig,
    upload_tx: mpsc::Sender<SegmentInfo>,
    state: Arc<RwLock<ServiceState>>,
) -> Result<()> {
    info!(camera_id = %camera.id, "Starting camera recorder");

    // Create temp directory for this camera
    let camera_temp_dir = recording.temp_dir.join(&camera.id);
    tokio::fs::create_dir_all(&camera_temp_dir)
        .await
        .context("Failed to create temp directory")?;

    let mut retry_count = 0;
    let max_retries = 5;
    let mut backoff_secs = 1u64;

    loop {
        match run_recording_session(&camera, &recording, &camera_temp_dir, &state, &upload_tx).await
        {
            Ok(()) => {
                warn!(camera_id = %camera.id, "Recording session ended normally");
                retry_count = 0;
                backoff_secs = 1;
            }
            Err(e) => {
                error!(
                    camera_id = %camera.id,
                    error = %e,
                    retry = retry_count,
                    "Recording session failed"
                );

                // Update connection state and metrics
                {
                    let mut s = state.write().await;
                    if s.cameras_connected > 0 {
                        s.cameras_connected -= 1;
                    }
                }
                metrics::CAMERA_CONNECTED
                    .with_label_values(&[&camera.id])
                    .set(0.0);
                metrics::FFMPEG_RESTARTS
                    .with_label_values(&[&camera.id])
                    .inc();

                retry_count += 1;
                if retry_count >= max_retries {
                    error!(
                        camera_id = %camera.id,
                        "Max retries exceeded, resetting backoff"
                    );
                    retry_count = 0;
                    backoff_secs = 1;
                }

                // Exponential backoff
                info!(
                    camera_id = %camera.id,
                    wait_secs = backoff_secs,
                    "Waiting before retry"
                );
                sleep(Duration::from_secs(backoff_secs)).await;
                backoff_secs = (backoff_secs * 2).min(60); // Max 60 second backoff
            }
        }
    }
}

/// Run a single recording session (until error or signal)
async fn run_recording_session(
    camera: &CameraConfig,
    recording: &RecordingConfig,
    temp_dir: &Path,
    state: &Arc<RwLock<ServiceState>>,
    upload_tx: &mpsc::Sender<SegmentInfo>,
) -> Result<()> {
    info!(camera_id = %camera.id, "Starting recording session");

    // Build output pattern for FFmpeg segmentation
    let output_pattern = temp_dir.join(format!("%Y%m%d_%H%M%S_{}.mp4", camera.id));

    let mut cmd = tokio::process::Command::new("ffmpeg");
    cmd.args([
        "-rtsp_transport",
        "tcp",
        "-i",
        &camera.rtsp_url,
        "-c:v",
        &recording.video_codec,
        "-c:a",
        &recording.audio_codec,
        "-f",
        "segment",
        "-segment_time",
        &camera.segment_duration_secs.to_string(),
        "-segment_format",
        "mp4",
        "-segment_format_options",
        "movflags=+frag_keyframe+empty_moov+default_base_moof",  // Enable streaming while recording
        "-strftime",
        "1",
        "-reset_timestamps",
        "1",
        "-y", // Overwrite files
        output_pattern.to_str().context("Invalid output path")?,
    ]);

    info!(camera_id = %camera.id, "Starting FFmpeg process");

    // Spawn FFmpeg with piped stderr
    let mut child = cmd
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn FFmpeg")?;

    // Mark camera as connected
    {
        let mut s = state.write().await;
        s.cameras_connected += 1;
    }
    metrics::CAMERA_CONNECTED
        .with_label_values(&[&camera.id])
        .set(1.0);

    info!(camera_id = %camera.id, "Camera connected, recording started");

    // Read stderr in background to detect segments
    let stderr = child.stderr.take().context("Failed to get stderr")?;
    let camera_id = camera.id.clone();
    let upload_tx_clone = upload_tx.clone();
    let temp_dir_clone = temp_dir.to_path_buf();

    let stderr_task = tokio::spawn(async move {
        parse_ffmpeg_stderr(stderr, camera_id, temp_dir_clone, upload_tx_clone).await;
    });

    // Wait for FFmpeg to complete
    let status = child.wait().await.context("FFmpeg process error")?;

    // Wait for stderr parsing to finish
    let _ = stderr_task.await;

    // Update connection state
    {
        let mut s = state.write().await;
        if s.cameras_connected > 0 {
            s.cameras_connected -= 1;
        }
    }
    metrics::CAMERA_CONNECTED
        .with_label_values(&[&camera.id])
        .set(0.0);

    if !status.success() {
        anyhow::bail!("FFmpeg exited with code: {:?}", status.code());
    }

    Ok(())
}

/// Parse FFmpeg stderr to detect completed segments
async fn parse_ffmpeg_stderr(
    stderr: impl tokio::io::AsyncRead + Unpin,
    camera_id: String,
    temp_dir: PathBuf,
    upload_tx: mpsc::Sender<SegmentInfo>,
) {
    let reader = BufReader::new(stderr);
    let mut lines = reader.lines();

    // Regex to match segment opening lines
    // Example: [segment @ 0x...] Opening 'output_20251214_013000_camera-1.mp4' for writing
    let segment_regex = Regex::new(r"Opening '([^']+)' for writing").unwrap();

    let mut current_segment: Option<String> = None;

    while let Ok(Some(line)) = lines.next_line().await {
        // Log FFmpeg output for debugging
        if line.contains("error") || line.contains("Error") {
            error!(camera_id = %camera_id, ffmpeg_output = %line, "FFmpeg error");
        }

        // Detect new segment
        if let Some(caps) = segment_regex.captures(&line) {
            if let Some(filename_match) = caps.get(1) {
                let segment_filename = filename_match.as_str();

                // If we have a previous segment, it's now complete - queue for upload
                if let Some(prev_segment) = current_segment.take() {
                    let segment_path = temp_dir.join(&prev_segment);

                    // Check if file exists and get size
                    if let Ok(metadata) = tokio::fs::metadata(&segment_path).await {
                        let size_bytes = metadata.len();

                        info!(
                            camera_id = %camera_id,
                            segment = %prev_segment,
                            size_mb = size_bytes / 1_048_576,
                            "Segment completed, queuing for upload"
                        );

                        metrics::SEGMENTS_RECORDED
                            .with_label_values(&[&camera_id])
                            .inc();
                        metrics::RECORDING_BYTES
                            .with_label_values(&[&camera_id])
                            .inc_by(size_bytes as f64);

                        let segment_info = SegmentInfo {
                            camera_id: camera_id.clone(),
                            local_path: segment_path,
                            timestamp: Utc::now(),
                        };

                        if let Err(e) = upload_tx.send(segment_info).await {
                            error!(error = %e, "Failed to send segment to upload queue");
                        }
                    }
                }

                // Track new segment
                current_segment = Some(
                    PathBuf::from(segment_filename)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                );

                info!(
                    camera_id = %camera_id,
                    segment = %current_segment.as_ref().unwrap(),
                    "Started new segment"
                );
            }
        }
    }

    // Handle last segment if stream ends
    if let Some(last_segment) = current_segment {
        let segment_path = temp_dir.join(&last_segment);

        if let Ok(metadata) = tokio::fs::metadata(&segment_path).await {
            let size_bytes = metadata.len();

            info!(
                camera_id = %camera_id,
                segment = %last_segment,
                size_mb = size_bytes / 1_048_576,
                "Final segment completed"
            );

            metrics::SEGMENTS_RECORDED
                .with_label_values(&[&camera_id])
                .inc();
            metrics::RECORDING_BYTES
                .with_label_values(&[&camera_id])
                .inc_by(size_bytes as f64);

            let segment_info = SegmentInfo {
                camera_id: camera_id.clone(),
                local_path: segment_path,
                timestamp: Utc::now(),
            };

            let _ = upload_tx.send(segment_info).await;
        }
    }
}
