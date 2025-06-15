use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, head, put, delete, post, patch},
    Json, Router,
};
use bytes::Bytes;
use opentelemetry::metrics::{Counter, Histogram};
use serde::Deserialize;
use tracing::{info, error, instrument};

use crate::error::{AppError, Result};
use crate::storage::Storage;
use super::models::{CatalogResponse, TagsListResponse};

// Application state with storage and metrics
pub struct AppMetrics {
    pub request_counter: Counter<u64>,
    pub blob_size_histogram: Histogram<f64>,
}

// Type alias for our application state
pub type AppState = (Arc<Storage>, Arc<AppMetrics>);

// Query parameters for catalog endpoint
#[derive(Debug, Deserialize)]
pub struct CatalogQuery {
    n: Option<usize>,
    last: Option<String>,
}

// Query parameters for tags list endpoint
#[derive(Debug, Deserialize)]
pub struct TagsQuery {
    n: Option<usize>,
    last: Option<String>,
}

// Create the main router for the registry API
pub fn registry_router(state: AppState) -> Router<AppState> {
    Router::new()
        // API Version Check
        .route("/v2/", get(api_version_check))

        // Catalog operations
        .route("/v2/_catalog", get(list_repositories))

        // Tag operations
        .route("/v2/{name}/tags/list", get(list_tags))

        // Manifest operations
        .route("/v2/{name}/manifests/{reference}", get(get_manifest))
        .route("/v2/{name}/manifests/{reference}", head(check_manifest))
        .route("/v2/{name}/manifests/{reference}", put(put_manifest))
        .route("/v2/{name}/manifests/{reference}", delete(delete_manifest))

        // Blob operations
        .route("/v2/{name}/blobs/{digest}", get(get_blob))
        .route("/v2/{name}/blobs/{digest}", head(check_blob))
        .route("/v2/{name}/blobs/{digest}", delete(delete_blob))

        // Blob upload operations
        .route("/v2/{name}/blobs/uploads/", post(start_upload))
        .route("/v2/{name}/blobs/uploads/{uuid}", get(get_upload_status))
        .route("/v2/{name}/blobs/uploads/{uuid}", patch(upload_chunk))
        .route("/v2/{name}/blobs/uploads/{uuid}", put(complete_upload))
        .route("/v2/{name}/blobs/uploads/{uuid}", delete(cancel_upload))
        .with_state(state)
}

// API Version Check
#[instrument(name = "api_version_check", skip_all)]
async fn api_version_check(
    State((_, metrics)): State<AppState>,
) -> impl IntoResponse {
    // Increment request counter
    metrics.request_counter.add(1, &[]);

    info!("API version check");
    StatusCode::OK
}

// List repositories
#[instrument(name = "list_repositories", skip(params, metrics), fields(n = ?params.n, last = ?params.last))]
async fn list_repositories(
    State((storage, metrics)): State<AppState>,
    Query(params): Query<CatalogQuery>,
) -> Result<Json<CatalogResponse>> {
    // Increment request counter
    metrics.request_counter.add(1, &[]);

    info!("Listing repositories");
    let mut repositories = storage.list_repositories().await?;

    // Apply pagination if requested
    if let Some(last) = &params.last {
        if let Some(pos) = repositories.iter().position(|r| r == last) {
            repositories = repositories.into_iter().skip(pos + 1).collect();
        }
    }

    if let Some(n) = params.n {
        repositories.truncate(n);
    }

    info!("Found {} repositories", repositories.len());
    Ok(Json(CatalogResponse { repositories }))
}

// List tags
#[instrument(name = "list_tags", skip(params, metrics), fields(repository = %name, n = ?params.n, last = ?params.last))]
async fn list_tags(
    State((storage, metrics)): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<TagsQuery>,
) -> Result<Json<TagsListResponse>> {
    // Increment request counter
    metrics.request_counter.add(1, &[]);

    info!("Listing tags for repository: {}", name);
    let mut tags = storage.list_tags(&name).await?;

    // Apply pagination if requested
    if let Some(last) = &params.last {
        if let Some(pos) = tags.iter().position(|t| t == last) {
            tags = tags.into_iter().skip(pos + 1).collect();
        }
    }

    if let Some(n) = params.n {
        tags.truncate(n);
    }

    info!("Found {} tags for repository: {}", tags.len(), name);
    Ok(Json(TagsListResponse { name, tags }))
}

// Get manifest
#[instrument(name = "get_manifest", skip(headers, metrics), fields(repository = %name, reference = %reference))]
async fn get_manifest(
    State((storage, metrics)): State<AppState>,
    Path((name, reference)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response> {
    // Increment request counter
    metrics.request_counter.add(1, &[]);

    info!("Getting manifest: {}/{}", name, reference);

    // Check if manifest exists
    if !storage.manifest_exists(&name, &reference).await? {
        error!("Manifest not found: {}/{}", name, reference);
        return Err(AppError::NotFound(format!("Manifest not found: {}/{}", name, reference)));
    }

    // Get the manifest content
    let content = storage.get_manifest(&name, &reference).await?;

    // Calculate digest
    let digest = format!("sha256:{}", sha256_digest(&content));

    // Record manifest size in histogram
    let content_length = content.len();
    metrics.blob_size_histogram.record(content_length as f64, &[]);

    info!("Retrieved manifest: {}/{}, size: {} bytes, digest: {}", 
          name, reference, content_length, digest);

    // Determine content type based on Accept header or default to OCI manifest
    let content_type = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/vnd.oci.image.manifest.v1+json");

    // Build response
    let mut response = Response::new(content.into());
    let headers = response.headers_mut();

    headers.insert(header::CONTENT_TYPE, content_type.parse().unwrap());
    headers.insert(header::CONTENT_LENGTH, content_length.into());
    headers.insert("Docker-Content-Digest", digest.parse().unwrap());

    Ok(response)
}

// Check manifest existence
#[instrument(name = "check_manifest", skip(metrics), fields(repository = %name, reference = %reference))]
async fn check_manifest(
    State((storage, metrics)): State<AppState>,
    Path((name, reference)): Path<(String, String)>,
) -> Result<StatusCode> {
    // Increment request counter
    metrics.request_counter.add(1, &[]);

    info!("Checking manifest: {}/{}", name, reference);

    if storage.manifest_exists(&name, &reference).await? {
        info!("Manifest exists: {}/{}", name, reference);
        Ok(StatusCode::OK)
    } else {
        error!("Manifest not found: {}/{}", name, reference);
        Err(AppError::NotFound(format!("Manifest not found: {}/{}", name, reference)))
    }
}

// Put manifest
#[instrument(name = "put_manifest", skip(body, metrics), fields(repository = %name, reference = %reference))]
async fn put_manifest(
    State((storage, metrics)): State<AppState>,
    Path((name, reference)): Path<(String, String)>,
    body: Bytes,
) -> Result<Response> {
    // Increment request counter
    metrics.request_counter.add(1, &[]);

    let body_size = body.len();
    info!("Putting manifest: {}/{}, size: {} bytes", name, reference, body_size);

    // Record manifest size in histogram
    metrics.blob_size_histogram.record(body_size as f64, &[]);

    // Store the manifest
    storage.put_manifest(&name, &reference, body.clone()).await?;

    // Calculate digest
    let digest = format!("sha256:{}", sha256_digest(&body));
    info!("Stored manifest: {}/{}, digest: {}", name, reference, digest);

    // Build response
    let mut response = Response::new(());
    let headers_map = response.headers_mut();

    headers_map.insert("Docker-Content-Digest", digest.parse().unwrap());
    headers_map.insert(header::LOCATION, format!("/v2/{}/manifests/{}", name, reference).parse().unwrap());

    *response.status_mut() = StatusCode::CREATED;

    Ok(empty_response_to_body(response))
}

// Delete manifest
#[instrument(name = "delete_manifest", skip(metrics), fields(repository = %name, reference = %reference))]
async fn delete_manifest(
    State((storage, metrics)): State<AppState>,
    Path((name, reference)): Path<(String, String)>,
) -> Result<StatusCode> {
    // Increment request counter
    metrics.request_counter.add(1, &[]);

    info!("Deleting manifest: {}/{}", name, reference);

    // Check if manifest exists
    if !storage.manifest_exists(&name, &reference).await? {
        error!("Manifest not found: {}/{}", name, reference);
        return Err(AppError::NotFound(format!("Manifest not found: {}/{}", name, reference)));
    }

    // Delete the manifest
    storage.delete_manifest(&name, &reference).await?;

    info!("Deleted manifest: {}/{}", name, reference);

    Ok(StatusCode::ACCEPTED)
}

// Get blob
#[instrument(name = "get_blob", skip(metrics), fields(repository = %name, digest = %digest))]
async fn get_blob(
    State((storage, metrics)): State<AppState>,
    Path((name, digest)): Path<(String, String)>,
) -> Result<Response> {
    // Increment request counter
    metrics.request_counter.add(1, &[]);

    info!("Getting blob: {}/{}", name, digest);

    // Check if blob exists
    if !storage.blob_exists(&digest).await? {
        error!("Blob not found: {}/{}", name, digest);
        return Err(AppError::NotFound(format!("Blob not found: {}", digest)));
    }

    // Get the blob content
    let content = storage.get_blob(&digest).await?;

    // Get content length before moving content
    let content_length = content.len();

    // Record blob size in histogram
    metrics.blob_size_histogram.record(content_length as f64, &[]);

    info!("Retrieved blob: {}/{}, size: {} bytes", name, digest, content_length);

    // Build response
    let mut response = Response::new(content.into());
    let headers = response.headers_mut();

    headers.insert(header::CONTENT_TYPE, "application/octet-stream".parse().unwrap());
    headers.insert(header::CONTENT_LENGTH, content_length.into());
    headers.insert("Docker-Content-Digest", digest.parse().unwrap());

    Ok(response)
}

// Check blob existence
#[instrument(name = "check_blob", skip(metrics), fields(repository = %name, digest = %digest))]
async fn check_blob(
    State((storage, metrics)): State<AppState>,
    Path((name, digest)): Path<(String, String)>,
) -> Result<StatusCode> {
    // Increment request counter
    metrics.request_counter.add(1, &[]);

    info!("Checking blob: {}/{}", name, digest);

    if storage.blob_exists(&digest).await? {
        info!("Blob exists: {}/{}", name, digest);
        Ok(StatusCode::OK)
    } else {
        error!("Blob not found: {}/{}", name, digest);
        Err(AppError::NotFound(format!("Blob not found: {}", digest)))
    }
}

// Delete blob
#[instrument(name = "delete_blob", skip(metrics), fields(repository = %name, digest = %digest))]
async fn delete_blob(
    State((storage, metrics)): State<AppState>,
    Path((name, digest)): Path<(String, String)>,
) -> Result<StatusCode> {
    // Increment request counter
    metrics.request_counter.add(1, &[]);

    info!("Deleting blob: {}/{}", name, digest);

    // Check if blob exists
    if !storage.blob_exists(&digest).await? {
        error!("Blob not found: {}/{}", name, digest);
        return Err(AppError::NotFound(format!("Blob not found: {}", digest)));
    }

    // Delete the blob
    storage.delete_blob(&digest).await?;

    info!("Deleted blob: {}/{}", name, digest);

    Ok(StatusCode::ACCEPTED)
}

// Start blob upload
#[instrument(name = "start_upload", skip(metrics), fields(repository = %name))]
async fn start_upload(
    State((storage, metrics)): State<AppState>,
    Path(name): Path<String>,
) -> Result<Response> {
    // Increment request counter
    metrics.request_counter.add(1, &[]);

    // Generate a session UUID for the upload
    let uuid = uuid::Uuid::new_v4().to_string();

    info!("Starting upload: {}, uuid: {}", name, uuid);

    storage.start_upload(&name, &uuid).await?;

    // Build response
    let mut response = Response::new(());
    let headers = response.headers_mut();

    headers.insert(header::LOCATION, format!("/v2/{}/blobs/uploads/{}", name, uuid).parse().unwrap());
    headers.insert(header::RANGE, "0-0".parse().unwrap());

    *response.status_mut() = StatusCode::ACCEPTED;

    info!("Upload started: {}, uuid: {}", name, uuid);

    Ok(empty_response_to_body(response))
}

// Get upload status
#[instrument(name = "get_upload_status", skip(metrics), fields(repository = %name, uuid = %uuid))]
async fn get_upload_status(
    State((_storage, metrics)): State<AppState>,
    Path((name, uuid)): Path<(String, String)>,
) -> Result<Response> {
    // Increment request counter
    metrics.request_counter.add(1, &[]);

    info!("Checking upload status: {}/{}", name, uuid);

    //storage.

    let mut response = Response::new(());
    let headers = response.headers_mut();

    headers.insert(header::LOCATION, format!("/v2/{}/blobs/uploads/{}", name, uuid).parse().unwrap());
    headers.insert(header::RANGE, "0-0".parse().unwrap());

    *response.status_mut() = StatusCode::ACCEPTED;

    info!("Upload status: {}/{} is in progress", name, uuid);

    Ok(empty_response_to_body(response))
}

// Upload blob chunk
#[instrument(name = "upload_chunk", skip(body, metrics), fields(repository = %name, uuid = %uuid))]
async fn upload_chunk(
    State((_storage, metrics)): State<AppState>,
    Path((name, uuid)): Path<(String, String)>,
    body: Bytes,
) -> Result<Response> {
    // Increment request counter
    metrics.request_counter.add(1, &[]);

    let body_size = body.len();
    info!("Uploading chunk: {}/{}, size: {} bytes", name, uuid, body_size);

    //TODO if we want to get compliant. we implement this function here
    // For our own use cases monolithic upload works

    let mut response = Response::new(());
    let headers_map = response.headers_mut();

    headers_map.insert(header::LOCATION, format!("/v2/{}/blobs/uploads/{}", name, uuid).parse().unwrap());
    headers_map.insert(header::RANGE, format!("0-{}", body_size).parse().unwrap());

    *response.status_mut() = StatusCode::ACCEPTED;

    info!("Chunk uploaded: {}/{}, size: {} bytes", name, uuid, body_size);

    Ok(empty_response_to_body(response))
}

// Complete upload
#[instrument(name = "complete_upload", skip(params, body, metrics), fields(repository = %name, uuid = %uuid))]
async fn complete_upload(
    State((storage, metrics)): State<AppState>,
    Path((name, uuid)): Path<(String, String)>,
    Query(params): Query<CompleteUploadQuery>,
    body: Bytes,
) -> Result<Response> {
    // Increment request counter
    metrics.request_counter.add(1, &[]);

    // Get the digest from query parameters
    let digest = params.digest.ok_or_else(|| AppError::BadRequest("Missing digest parameter".to_string()))?;

    let body_size = body.len();
    info!("Completing upload: {}/{}, uuid: {}, digest: {}, size: {} bytes", 
          name, digest, uuid, digest, body_size);

    // Record blob size in histogram
    metrics.blob_size_histogram.record(body_size as f64, &[]);

    // Store the blob
    storage.put_blob(&digest, body).await?;

    info!("Completed upload: {}/{}, uuid: {}, digest: {}", name, digest, uuid, digest);

    // Build response
    let mut response = Response::new(());
    let headers = response.headers_mut();

    headers.insert(header::LOCATION, format!("/v2/{}/blobs/{}", name, digest).parse().unwrap());
    headers.insert("Docker-Content-Digest", digest.parse().unwrap());

    *response.status_mut() = StatusCode::CREATED;

    Ok(empty_response_to_body(response))
}

// Cancel upload
#[instrument(name = "cancel_upload", skip(metrics), fields(repository = %name, uuid = %uuid))]
async fn cancel_upload(
    State((_storage, metrics)): State<AppState>,
    Path((name, uuid)): Path<(String, String)>,
) -> Result<StatusCode> {
    // Increment request counter
    metrics.request_counter.add(1, &[]);

    info!("Cancelling upload: {}/{}", name, uuid);

    // In a real implementation, we would delete the upload
    // For simplicity, we'll just return a success response

    info!("Upload cancelled: {}/{}", name, uuid);

    Ok(StatusCode::ACCEPTED)
}

// Query parameters for complete upload
#[derive(Debug, Deserialize)]
struct CompleteUploadQuery {
    digest: Option<String>,
}

// Helper function to calculate SHA256 digest
fn sha256_digest(data: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

// Helper function to convert Response<()> to Response<Body>
fn empty_response_to_body(response: Response<()>) -> Response<Body> {
    let (parts, _) = response.into_parts();
    Response::from_parts(parts, Body::empty())
}
