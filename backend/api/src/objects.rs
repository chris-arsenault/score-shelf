//! Private S3 access through presigned URLs. File bytes never pass through
//! the API: the shared ALB's WAF and the Lambda payload limit both forbid it.

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_s3::presigning::PresigningConfig;
use serde::Serialize;

use crate::error::{AppError, AppResult};

pub const UPLOAD_EXPIRY_SECONDS: u64 = 15 * 60;
pub const DOWNLOAD_EXPIRY_SECONDS: u64 = 10 * 60;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UploadTarget {
    pub url: String,
    pub method: &'static str,
    pub headers: HashMap<String, String>,
}

#[async_trait]
pub trait ObjectStore: Send + Sync {
    async fn upload_target(&self, key: &str, content_type: &str) -> AppResult<UploadTarget>;
    async fn download_url(
        &self,
        key: &str,
        content_type: &str,
        filename: &str,
    ) -> AppResult<String>;
    /// The stored object's size, or `None` when it has not been uploaded.
    async fn object_size(&self, key: &str) -> AppResult<Option<i64>>;
}

pub struct S3ObjectStore {
    s3: aws_sdk_s3::Client,
    bucket: String,
}

impl S3ObjectStore {
    pub async fn new(bucket: String) -> Self {
        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        Self {
            s3: aws_sdk_s3::Client::new(&config),
            bucket,
        }
    }
}

#[async_trait]
impl ObjectStore for S3ObjectStore {
    async fn upload_target(&self, key: &str, content_type: &str) -> AppResult<UploadTarget> {
        let presigned = self
            .s3
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .presigned(presigning(UPLOAD_EXPIRY_SECONDS)?)
            .await
            .map_err(s3_error)?;
        Ok(UploadTarget {
            url: presigned.uri().to_string(),
            method: "PUT",
            headers: HashMap::from([("content-type".to_string(), content_type.to_string())]),
        })
    }

    async fn download_url(
        &self,
        key: &str,
        content_type: &str,
        filename: &str,
    ) -> AppResult<String> {
        let presigned = self
            .s3
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .response_content_type(content_type)
            .response_content_disposition(content_disposition(filename))
            .presigned(presigning(DOWNLOAD_EXPIRY_SECONDS)?)
            .await
            .map_err(s3_error)?;
        Ok(presigned.uri().to_string())
    }

    async fn object_size(&self, key: &str) -> AppResult<Option<i64>> {
        match self
            .s3
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(head) => Ok(head.content_length()),
            Err(err) if err.as_service_error().is_some_and(|e| e.is_not_found()) => Ok(None),
            Err(err) => Err(s3_error(err)),
        }
    }
}

pub fn content_disposition(filename: &str) -> String {
    let safe: String = filename
        .chars()
        .map(|c| if c == '"' || c == '\\' { '_' } else { c })
        .collect();
    format!("attachment; filename=\"{safe}\"")
}

fn presigning(seconds: u64) -> AppResult<PresigningConfig> {
    PresigningConfig::expires_in(Duration::from_secs(seconds))
        .map_err(|err| AppError::Config(err.to_string()))
}

fn s3_error(err: impl std::fmt::Display) -> AppError {
    AppError::ExternalService {
        service: "s3",
        message: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_disposition_names_the_file_safely() {
        assert_eq!(
            content_disposition("boreal_pocket.musicxml"),
            "attachment; filename=\"boreal_pocket.musicxml\""
        );
        assert_eq!(
            content_disposition("a\"b.mid"),
            "attachment; filename=\"a_b.mid\""
        );
    }

    #[test]
    fn test_presigning_accepts_both_expiries() {
        assert!(presigning(UPLOAD_EXPIRY_SECONDS).is_ok());
        assert!(presigning(DOWNLOAD_EXPIRY_SECONDS).is_ok());
    }
}
