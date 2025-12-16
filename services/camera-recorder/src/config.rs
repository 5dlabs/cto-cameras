use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub service: ServiceConfig,
    pub storage: StorageConfig,
    pub cameras: Vec<CameraConfig>,
    pub recording: RecordingConfig,
    pub upload: UploadConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    pub metrics_port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CameraConfig {
    pub id: String,
    pub name: String,
    pub rtsp_url: String,
    pub segment_duration_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecordingConfig {
    pub temp_dir: PathBuf,
    pub local_retention_minutes: u64,
    pub video_codec: String,
    pub audio_codec: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadConfig {
    pub max_concurrent: usize,
    pub max_retries: u32,
    pub retry_backoff_secs: u64,
}

impl Config {
    /// Load configuration from TOML file
    pub fn from_file(path: &str) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).context(format!("Failed to read config file: {path}"))?;
        let config: Config = toml::from_str(&content).context("Failed to parse config file")?;
        config.validate()?;
        Ok(config)
    }

    /// Load from environment variables (for Kubernetes)
    pub fn from_env() -> Result<Self> {
        let config = Config {
            service: ServiceConfig {
                metrics_port: std::env::var("METRICS_PORT")
                    .unwrap_or_else(|_| "9090".to_string())
                    .parse()?,
            },
            storage: StorageConfig {
                endpoint: std::env::var("S3_ENDPOINT").context("S3_ENDPOINT not set")?,
                bucket: std::env::var("S3_BUCKET")
                    .unwrap_or_else(|_| "camera-recordings".to_string()),
                region: std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
                access_key_id: std::env::var("S3_ACCESS_KEY_ID")
                    .context("S3_ACCESS_KEY_ID not set")?,
                secret_access_key: std::env::var("S3_SECRET_ACCESS_KEY")
                    .context("S3_SECRET_ACCESS_KEY not set")?,
            },
            cameras: vec![
                CameraConfig {
                    id: "camera-1".to_string(),
                    name: "Camera 1".to_string(),
                    rtsp_url: std::env::var("CAMERA1_RTSP_URL")
                        .context("CAMERA1_RTSP_URL not set")?,
                    segment_duration_secs: 900, // 15 minutes
                },
                CameraConfig {
                    id: "camera-2".to_string(),
                    name: "Camera 2".to_string(),
                    rtsp_url: std::env::var("CAMERA2_RTSP_URL")
                        .context("CAMERA2_RTSP_URL not set")?,
                    segment_duration_secs: 900,
                },
            ],
            recording: RecordingConfig {
                temp_dir: PathBuf::from(
                    std::env::var("TEMP_DIR")
                        .unwrap_or_else(|_| "/tmp/camera-recordings".to_string()),
                ),
                local_retention_minutes: 60,
                video_codec: "copy".to_string(),
                audio_codec: "aac".to_string(),
            },
            upload: UploadConfig {
                max_concurrent: std::env::var("MAX_CONCURRENT_UPLOADS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(4),
                max_retries: 5,
                retry_backoff_secs: 5,
            },
        };
        config.validate()?;
        Ok(config)
    }

    /// Validate configuration
    fn validate(&self) -> Result<()> {
        anyhow::ensure!(!self.cameras.is_empty(), "No cameras configured");
        anyhow::ensure!(
            !self.storage.endpoint.is_empty(),
            "Storage endpoint not configured"
        );
        anyhow::ensure!(
            !self.storage.bucket.is_empty(),
            "Storage bucket not configured"
        );
        Ok(())
    }
}
