use std::time::Duration;

use aws_config::BehaviorVersion;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client;

use crate::config::StorageConfig;
use crate::upload_api::UploadHeader;

#[derive(Clone)]
pub struct Storage {
    bucket: String,
    client: Client,
}

#[derive(Clone, Debug)]
pub struct PresignedUpload {
    pub url: String,
    pub headers: Vec<UploadHeader>,
}

impl Storage {
    pub async fn connect(config: &StorageConfig) -> Self {
        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()))
            .load()
            .await;
        let client = Client::new(&aws_config);

        Self {
            bucket: config.bucket.clone(),
            client,
        }
    }

    pub async fn presign_original_upload(
        &self,
        key: &str,
        mime_type: &str,
    ) -> anyhow::Result<PresignedUpload> {
        let request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(mime_type)
            .tagging("keep_hot=false")
            .presigned(presign_config()?)
            .await?;

        Ok(PresignedUpload {
            url: request.uri().to_string(),
            headers: upload_headers(&request),
        })
    }

    pub async fn presign_metadata_upload(&self, key: &str) -> anyhow::Result<PresignedUpload> {
        let request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type("application/json")
            .presigned(presign_config()?)
            .await?;

        Ok(PresignedUpload {
            url: request.uri().to_string(),
            headers: upload_headers(&request),
        })
    }

    pub async fn object_exists(&self, key: &str) -> anyhow::Result<bool> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(error)
                if error
                    .as_service_error()
                    .is_some_and(|error| error.is_not_found()) =>
            {
                Ok(false)
            }
            Err(error) => Err(error.into()),
        }
    }
}

fn presign_config() -> anyhow::Result<PresigningConfig> {
    Ok(PresigningConfig::expires_in(Duration::from_secs(15 * 60))?)
}

fn upload_headers(request: &aws_sdk_s3::presigning::PresignedRequest) -> Vec<UploadHeader> {
    request
        .headers()
        .map(|(name, value)| UploadHeader {
            name: name.to_string(),
            value: value.to_string(),
        })
        .collect()
}
