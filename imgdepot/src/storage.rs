use crate::config::{AppConfig, StorageBackend};
use crate::error::{AppError, Result};
use opendal::services::Fs;
use opendal::services::S3;
use opendal::Operator;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadStatus {
    name: String,
    uuid: String,
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
        let upload = UploadStatus { name: name.to_string(), uuid: uuid.to_string() };
        let upload_serialized = serde_json::to_vec(&upload)?;
        self.operator.write(path.as_str(), upload_serialized).await?;
        self.operator.create_dir(chunks_path.as_str()).await?;
        Ok(())
    }

    pub async fn get_upload_status(&self, name: &str, uuid: &str) -> Result<bool> {
        let path = format!("uploads/{name}/{uuid}-status.json");
        self.operator.is_exist(&path)
            .await
            .map_err(AppError::Storage)
    }
    
    // pub async fn upload_chunk(&self, name: &str, uuid: &str, content: bytes::Bytes) -> Result<()> {
    //     let path = format!("uploads/{name}/{uuid}/chunks/{size}");
    // }

    pub async fn blob_exists(&self, digest: &str) -> Result<bool> {
        let path = format!("blobs/{}", digest);
        self.operator.is_exist(&path)
            .await
            .map_err(AppError::Storage)
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
