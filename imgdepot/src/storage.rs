use crate::config::{AppConfig, StorageBackend};
use crate::error::{AppError, Result};
use opendal::services::Fs;
use opendal::services::S3;
use opendal::Operator;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadStatus {
    pub name: String,
    pub uuid: String,
    pub size: u64,
}

#[derive(Debug)]
pub struct Storage {
    operator: Operator,
}

impl Storage {
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let operator = match config.storage.backend {
            StorageBackend::Fs => {
                let root = config.storage.fs_root.clone()
                    .ok_or_else(|| AppError::Config("Missing fs_root configuration".to_string()))?;

                let mut builder = Fs::default();
                builder.root(&root.to_string_lossy());

                Operator::new(builder)
                    .map_err(AppError::Storage)?
                    .finish()
            }
            StorageBackend::S3 => {
                let bucket = config.storage.s3_bucket.clone()
                    .ok_or_else(|| AppError::Config("Missing s3_bucket configuration".to_string()))?;
                let region = config.storage.s3_region.clone()
                    .ok_or_else(|| AppError::Config("Missing s3_region configuration".to_string()))?;

                let mut builder = S3::default();
                builder.bucket(&bucket);
                builder.region(&region);

                if let Some(endpoint) = &config.storage.s3_endpoint {
                    builder.endpoint(endpoint);
                }

                if let Some(access_key) = &config.storage.s3_access_key {
                    builder.access_key_id(access_key);
                }

                if let Some(secret_key) = &config.storage.s3_secret_key {
                    builder.secret_access_key(secret_key);
                }

                Operator::new(builder)
                    .map_err(AppError::Storage)?
                    .finish()
            }
        };

        Ok(Self { operator })
    }

    // Blob operations

    pub async fn start_upload(&self, name: &str, uuid: &str) -> Result<()> {
        let path = format!("uploads/{name}/{uuid}-status.json");
        let chunks_path = format!("uploads/{name}/{uuid}/chunks");
        let upload = UploadStatus { 
            name: name.to_string(), 
            uuid: uuid.to_string(),
            size: 0,
        };
        let upload_serialized = serde_json::to_vec(&upload)?;
        self.operator.write(path.as_str(), upload_serialized).await?;
        self.operator.create_dir(chunks_path.as_str()).await?;

        // Create the temporary file for chunked uploads
        let temp_path = format!("uploads/{name}/{uuid}.part");
        self.operator.write(&temp_path, bytes::Bytes::new()).await?;

        Ok(())
    }

    pub async fn get_upload_status(&self, name: &str, uuid: &str) -> Result<UploadStatus> {
        let path = format!("uploads/{name}/{uuid}-status.json");

        if !self.operator.is_exist(&path).await.map_err(AppError::Storage)? {
            return Err(AppError::NotFound(format!("Upload not found: {}/{}", name, uuid)));
        }

        let data = self.operator.read(&path).await.map_err(AppError::Storage)?;
        let status: UploadStatus = serde_json::from_slice(&data)?;

        Ok(status)
    }

    pub async fn upload_chunk(&self, name: &str, uuid: &str, content: bytes::Bytes) -> Result<u64> {
        // Check if upload exists
        let status_path = format!("uploads/{name}/{uuid}-status.json");
        if !self.operator.is_exist(&status_path).await.map_err(AppError::Storage)? {
            return Err(AppError::NotFound(format!("Upload not found: {}/{}", name, uuid)));
        }

        // Get current upload status
        let data = self.operator.read(&status_path).await.map_err(AppError::Storage)?;
        let mut status: UploadStatus = serde_json::from_slice(&data)?;

        // Get the temporary file path
        let temp_path = format!("uploads/{name}/{uuid}.part");

        // Read the current content
        let current_content = if self.operator.is_exist(&temp_path).await.map_err(AppError::Storage)? {
            self.operator.read(&temp_path).await.map_err(AppError::Storage)?
        } else {
            Vec::new()
        };

        // Append the new chunk to the existing content
        let mut new_content = current_content;
        new_content.extend_from_slice(&content);

        // Write the combined content back to the temporary file
        self.operator.write(&temp_path, bytes::Bytes::from(new_content)).await.map_err(AppError::Storage)?;

        // Update the upload status with the new size
        status.size += content.len() as u64;
        let status_serialized = serde_json::to_vec(&status)?;
        self.operator.write(&status_path, status_serialized).await.map_err(AppError::Storage)?;

        Ok(status.size)
    }

    pub async fn blob_exists(&self, digest: &str) -> Result<bool> {
        let path = format!("blobs/{}", digest);
        self.operator.is_exist(&path)
            .await
            .map_err(AppError::Storage)
    }

    pub async fn get_blob_size(&self, digest: &str) -> Result<u64> {
        let path = format!("blobs/{}", digest);
        let metadata = self.operator.stat(&path)
            .await
            .map_err(AppError::Storage)?;
        Ok(metadata.content_length())
    }

    pub async fn get_blob(&self, digest: &str) -> Result<bytes::Bytes> {
        let path = format!("blobs/{}", digest);
        let data = self.operator.read(&path)
            .await
            .map_err(AppError::Storage)?;
        Ok(bytes::Bytes::from(data))
    }

    pub async fn put_blob(&self, digest: &str, content: bytes::Bytes) -> Result<()> {
        let path = format!("blobs/{}", digest);
        self.operator.write(&path, content)
            .await
            .map_err(AppError::Storage)
    }

    pub async fn complete_upload(&self, name: &str, uuid: &str, expected_digest: Option<&str>) -> Result<String> {
        // Check if upload exists
        let status_path = format!("uploads/{name}/{uuid}-status.json");
        if !self.operator.is_exist(&status_path).await.map_err(AppError::Storage)? {
            return Err(AppError::NotFound(format!("Upload not found: {}/{}", name, uuid)));
        }

        // Get the temporary file path
        let temp_path = format!("uploads/{name}/{uuid}.part");

        // Read the content of the temporary file
        let content = if self.operator.is_exist(&temp_path).await.map_err(AppError::Storage)? {
            self.operator.read(&temp_path).await.map_err(AppError::Storage)?
        } else {
            return Err(AppError::NotFound(format!("Upload file not found: {}/{}", name, uuid)));
        };

        // Calculate the digest
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let digest_calculated = format!("sha256:{}", hex::encode(hasher.finalize()));

        // If an expected digest was provided, verify it matches
        if let Some(expected) = expected_digest {
            if expected != digest_calculated {
                return Err(AppError::BadRequest(format!(
                    "Digest mismatch: expected {}, got {}", 
                    expected, 
                    digest_calculated
                )));
            }
        }

        // Store the blob with the calculated digest
        let blob_path = format!("blobs/{}", digest_calculated);
        self.operator.write(&blob_path, bytes::Bytes::from(content))
            .await
            .map_err(AppError::Storage)?;

        // Clean up the temporary files
        self.cancel_upload(name, uuid).await?;

        Ok(digest_calculated)
    }

    pub async fn cancel_upload(&self, name: &str, uuid: &str) -> Result<()> {
        // Get the paths for the temporary files
        let status_path = format!("uploads/{name}/{uuid}-status.json");
        let temp_path = format!("uploads/{name}/{uuid}.part");
        let chunks_path = format!("uploads/{name}/{uuid}/chunks");

        // Delete the temporary files if they exist
        if self.operator.is_exist(&temp_path).await.map_err(AppError::Storage)? {
            self.operator.delete(&temp_path).await.map_err(AppError::Storage)?;
        }

        if self.operator.is_exist(&status_path).await.map_err(AppError::Storage)? {
            self.operator.delete(&status_path).await.map_err(AppError::Storage)?;
        }

        // Delete the chunks directory if it exists
        if self.operator.is_exist(&chunks_path).await.map_err(AppError::Storage)? {
            // First delete all files in the chunks directory
            let entries = self.operator.list(&chunks_path).await.map_err(AppError::Storage)?;
            for entry in entries {
                let chunk_path = format!("{}/{}", chunks_path, entry.name());
                self.operator.delete(&chunk_path).await.map_err(AppError::Storage)?;
            }

            // Then delete the directory itself
            self.operator.delete(&chunks_path).await.map_err(AppError::Storage)?;
        }

        Ok(())
    }

    pub async fn delete_blob(&self, digest: &str) -> Result<()> {
        let path = format!("blobs/{}", digest);
        self.operator.delete(&path)
            .await
            .map_err(AppError::Storage)
    }

    // Manifest operations

    pub async fn manifest_exists(&self, repository: &str, reference: &str) -> Result<bool> {
        let path = format!("manifests/{}/{}", repository, reference);
        self.operator.is_exist(&path)
            .await
            .map_err(AppError::Storage)
    }

    pub async fn get_manifest(&self, repository: &str, reference: &str) -> Result<bytes::Bytes> {
        let path = format!("manifests/{}/{}", repository, reference);
        let data = self.operator.read(&path)
            .await
            .map_err(AppError::Storage)?;
        Ok(bytes::Bytes::from(data))
    }

    pub async fn put_manifest(&self, repository: &str, reference: &str, content: bytes::Bytes) -> Result<()> {
        let path = format!("manifests/{}/{}", repository, reference);
        self.operator.write(&path, content)
            .await
            .map_err(AppError::Storage)
    }

    pub async fn delete_manifest(&self, repository: &str, reference: &str) -> Result<()> {
        let path = format!("manifests/{}/{}", repository, reference);
        self.operator.delete(&path)
            .await
            .map_err(AppError::Storage)
    }

    // Repository operations

    pub async fn list_repositories(&self) -> Result<Vec<String>> {
        let path = "manifests/";
        let entries = self.operator.list(&path)
            .await
            .map_err(AppError::Storage)?;

        let mut repositories = Vec::new();
        for entry in entries {
            if entry.metadata().is_dir() {
                repositories.push(entry.name().to_string());
            }
        }

        Ok(repositories)
    }

    pub async fn list_tags(&self, repository: &str) -> Result<Vec<String>> {
        let path = format!("manifests/{}/", repository);
        let entries = self.operator.list(&path)
            .await
            .map_err(AppError::Storage)?;

        let mut tags = Vec::new();
        for entry in entries {
            if !entry.metadata().is_dir() {
                tags.push(entry.name().to_string());
            }
        }

        Ok(tags)
    }
}
