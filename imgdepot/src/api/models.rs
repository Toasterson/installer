use serde::{Deserialize, Serialize};

// OCI Distribution Spec Models

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub errors: Vec<ErrorInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
    pub detail: Option<serde_json::Value>,
}

// Catalog response
#[derive(Debug, Serialize, Deserialize)]
pub struct CatalogResponse {
    pub repositories: Vec<String>,
}

// Tags list response
#[derive(Debug, Serialize, Deserialize)]
pub struct TagsListResponse {
    pub name: String,
    pub tags: Vec<String>,
}

// Manifest response headers
#[derive(Debug, Serialize)]
pub struct ManifestResponseHeaders {
    pub docker_content_digest: String,
    pub content_type: String,
    pub content_length: usize,
}

// Upload status response
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadStatusResponse {
    pub uuid: String,
    pub offset: usize,
    pub range: Option<String>,
}

// Upload complete response
#[derive(Debug, Serialize)]
pub struct UploadCompleteHeaders {
    pub location: String,
    pub docker_content_digest: String,
}