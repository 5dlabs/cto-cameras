use crate::config::StorageConfig;
use anyhow::{Context, Result};
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Region, SharedCredentialsProvider};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use std::path::Path;
use tracing::info;

#[derive(Clone)]
pub struct S3Client {
    client: Client,
    bucket: String,
}

impl S3Client {
    /// Create new S3 client for SeaweedFS
    pub async fn new(config: &StorageConfig) -> Result<Self> {
        // Create credentials
        let credentials = Credentials::new(
            &config.access_key_id,
            &config.secret_access_key,
            None,
            None,
            "static",
        );

        // Build S3 config with path-style addressing for SeaweedFS compatibility
        let s3_config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()))
            .endpoint_url(&config.endpoint)
            .credentials_provider(SharedCredentialsProvider::new(credentials))
            .force_path_style(true)  // Required for SeaweedFS and other S3-compatible stores
            .build();

        let client = Client::from_conf(s3_config);

        Ok(Self {
            client,
            bucket: config.bucket.clone(),
        })
    }

    /// Ensure bucket exists, create if needed
    pub async fn ensure_bucket_exists(&self) -> Result<()> {
        match self.client.head_bucket().bucket(&self.bucket).send().await {
            Ok(_) => {
                info!(bucket = %self.bucket, "Bucket exists");
                Ok(())
            }
            Err(_) => {
                info!(bucket = %self.bucket, "Bucket not found, creating...");
                self.client
                    .create_bucket()
                    .bucket(&self.bucket)
                    .send()
                    .await
                    .context("Failed to create bucket")?;
                info!(bucket = %self.bucket, "Bucket created");
                Ok(())
            }
        }
    }

    /// Upload a file to S3
    pub async fn upload_file(&self, local_path: &Path, s3_key: &str) -> Result<()> {
        info!(
            local_path = %local_path.display(),
            s3_key = %s3_key,
            "Uploading file to SeaweedFS"
        );

        let body = ByteStream::from_path(local_path)
            .await
            .context("Failed to read file")?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(s3_key)
            .body(body)
            .send()
            .await
            .context("Failed to upload to S3")?;

        info!(s3_key = %s3_key, "Upload successful");
        Ok(())
    }

    /// Delete local file after successful upload
    pub async fn cleanup_local_file(&self, path: &Path) -> Result<()> {
        tokio::fs::remove_file(path)
            .await
            .context("Failed to delete local file")?;
        info!(path = %path.display(), "Cleaned up local file");
        Ok(())
    }
}
