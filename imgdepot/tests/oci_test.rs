use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time::sleep;

use opentelemetry::metrics::MeterProvider;

use oci_util::distribution::client::Registry;

use imgdepot::api::routes::AppMetrics;
use imgdepot::config::AppConfig;
use imgdepot::storage::Storage;

// Helper function to start the registry server for testing
async fn start_test_server() -> (JoinHandle<()>, u16) {
    // Use a random available port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    // Create a test configuration
    let config = AppConfig {
        port: port,
        storage: imgdepot::config::StorageConfig {
            backend: imgdepot::config::StorageBackend::Fs,
            fs_root: Some(std::path::PathBuf::from("./data")),
            s3_bucket: None,
            s3_region: None,
            s3_endpoint: None,
            s3_access_key: None,
            s3_secret_key: None,
        },
    };

    // Initialize storage
    let storage = Storage::new(&config).await.unwrap();
    let storage = Arc::new(storage);

    // Create metrics for testing
    let meter = opentelemetry::metrics::noop::NoopMeterProvider::new().meter("test");
    let app_metrics = Arc::new(AppMetrics {
        request_counter: meter.u64_counter("test_requests").init(),
        blob_size_histogram: meter.f64_histogram("test_blob_size").init(),
    });

    // Create application state
    let app_state = (Arc::clone(&storage), Arc::clone(&app_metrics));

    // Build application
    let app = axum::Router::new()
        .merge(imgdepot::api::routes::registry_router(app_state))
        .with_state((storage, app_metrics));

    // Start server in a separate task
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give the server a moment to start
    sleep(Duration::from_millis(100)).await;

    (server, port)
}

#[tokio::test]
async fn test_api_version_check_with_oci_util() {
    // Start the test server
    let (server, port) = start_test_server().await;

    // Create a registry client
    let registry = Registry::new(
        format!("http://localhost:{}", port),
        None, // No auth for testing
    );

    // Create a session
    let _session = registry.new_session("test".to_string());

    // Make a request to the API version endpoint using reqwest directly
    let response = reqwest::get(format!("http://localhost:{}/v2/", port))
        .await
        .unwrap();

    // Check that the response is successful
    assert_eq!(response.status().as_u16(), 200);

    // Shutdown the server
    server.abort();
}

#[tokio::test]
async fn test_blob_operations() {
    // Start the test server
    let (server, port) = start_test_server().await;

    // Create test content
    let content = "test blob content".as_bytes();

    // Calculate digest
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content);
    let digest = format!("sha256:{}", hex::encode(hasher.finalize()));

    // Start upload
    let client = reqwest::Client::new();
    let start_response = client.post(format!("http://localhost:{}/v2/test/blobs/uploads/", port))
        .send()
        .await
        .unwrap();

    assert_eq!(start_response.status().as_u16(), 202);

    // Get the upload location
    let location = start_response.headers().get("location").unwrap().to_str().unwrap();
    let upload_url = format!("http://localhost:{}{}", port, location);

    // Complete upload
    let complete_url = format!("{}?digest={}", upload_url, digest);
    let complete_response = client.put(complete_url)
        .body(content.to_vec())
        .send()
        .await
        .unwrap();

    assert_eq!(complete_response.status().as_u16(), 201, "Failed to complete upload: {:?}", complete_response);

    // Check blob exists
    let check_response = client.head(format!("http://localhost:{}/v2/test/blobs/{}", port, digest))
        .send()
        .await
        .unwrap();

    assert_eq!(check_response.status().as_u16(), 200, "Blob not found: {:?}", check_response);

    // Create a simple manifest that references the blob
    let manifest = serde_json::json!({
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "config": {
            "mediaType": "application/vnd.oci.image.config.v1+json",
            "digest": digest,
            "size": content.len()
        },
        "layers": [
            {
                "mediaType": "application/vnd.oci.image.layer.v1.tar",
                "digest": digest,
                "size": content.len()
            }
        ]
    });

    // Push the manifest
    let manifest_response = client.put(format!("http://localhost:{}/v2/test/manifests/latest", port))
        .header("Content-Type", "application/vnd.oci.image.manifest.v1+json")
        .json(&manifest)
        .send()
        .await
        .unwrap();

    assert_eq!(manifest_response.status().as_u16(), 201, "Failed to push manifest: {:?}", manifest_response);

    // List repositories
    let response = reqwest::get(format!("http://localhost:{}/v2/_catalog", port))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);

    let catalog: serde_json::Value = response.json().await.unwrap();
    println!("Catalog response: {}", serde_json::to_string_pretty(&catalog).unwrap());

    // Check if the manifest exists
    let manifest_check = client.head(format!("http://localhost:{}/v2/test/manifests/latest", port))
        .send()
        .await
        .unwrap();

    println!("Manifest check status: {}", manifest_check.status().as_u16());

    // Try to get the manifest
    let manifest_get = client.get(format!("http://localhost:{}/v2/test/manifests/latest", port))
        .send()
        .await
        .unwrap();

    println!("Manifest get status: {}", manifest_get.status().as_u16());
    if manifest_get.status().is_success() {
        println!("Manifest content: {}", manifest_get.text().await.unwrap());
    }

    let repositories = catalog["repositories"].as_array().unwrap();

    // Check that our test repository is in the list
    let found = repositories.iter().any(|repo| repo.as_str().unwrap() == "test/");
    assert!(found, "Repository 'test' not found in catalog");

    // Shutdown the server
    server.abort();
}

#[tokio::test]
async fn test_chunked_upload_with_oci_util() {
    // Start the test server
    let (_, port) = start_test_server().await;

    // Create a registry client
    let registry = Registry::new(
        format!("http://localhost:{}", port),
        None, // No auth for testing
    );

    // Create a session
    let mut session = registry.new_session("test".to_string());
    
    // Create test content - make it large enough to ensure multiple chunks
    let content = "test blob content".repeat(1000); // ~17KB of data
    let content_bytes = content.as_bytes();
    
    let cursor = std::io::Cursor::new(content_bytes);
    let descriptor = session.upload_content(None, "application/octet-stream".to_string(), cursor)
        .await.unwrap();
    
    println!("{:?}", descriptor);
    
}

#[tokio::test]
async fn test_chunked_upload() {
    // Start the test server
    let (server, port) = start_test_server().await;

    // First, let's verify the server is running by checking the API version
    let version_response = reqwest::get(format!("http://localhost:{}/v2/", port))
        .await
        .unwrap();
    assert_eq!(version_response.status().as_u16(), 200, "API version check failed");

    // Create test content - make it large enough to ensure multiple chunks
    let content = "test blob content".repeat(1000); // ~17KB of data
    let content_bytes = content.as_bytes();

    // Start upload using the same approach as test_blob_operations
    let client = reqwest::Client::new();
    let start_response = client.post(format!("http://localhost:{}/v2/test/blobs/uploads/", port))
        .send()
        .await
        .unwrap();

    // If the response is not successful, print the response body to help debug
    let status = start_response.status().as_u16();
    if status != 202 {
        let error_body = start_response.text().await.unwrap();
        println!("Error response body: {}", error_body);
        panic!("Failed to start upload: status {}", status);
    }

    // Get the upload location
    let location = start_response.headers().get("location").unwrap().to_str().unwrap();
    let upload_url = format!("http://localhost:{}{}", port, location);

    println!("Upload URL: {}", upload_url);

    // Calculate digest for verification
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content_bytes);
    let expected_digest = format!("sha256:{}", hex::encode(hasher.finalize()));

    // Upload chunks manually to simulate what upload_content does
    let chunk_size = 4096; // 4KB chunks
    let mut offset = 0;

    while offset < content_bytes.len() {
        let end = std::cmp::min(offset + chunk_size, content_bytes.len());
        let chunk = &content_bytes[offset..end];

        // Upload chunk
        let chunk_response = client.patch(&upload_url)
            .header("Content-Type", "application/octet-stream")
            .header("Content-Length", chunk.len())
            .header("Range", format!("{}-{}", offset, end - 1))
            .body(chunk.to_vec())
            .send()
            .await
            .unwrap();

        assert_eq!(chunk_response.status().as_u16(), 202, "Failed to upload chunk: {:?}", chunk_response);

        offset = end;
    }

    // Complete the upload
    let complete_url = format!("{}?digest={}", upload_url, expected_digest);
    let complete_response = client.put(complete_url)
        .send()
        .await
        .unwrap();

    assert_eq!(complete_response.status().as_u16(), 201, "Failed to complete upload: {:?}", complete_response);

    // Verify the upload was successful by checking the blob exists
    let check_response = client.head(format!("http://localhost:{}/v2/test/blobs/{}", port, expected_digest))
        .send()
        .await
        .unwrap();

    assert_eq!(check_response.status().as_u16(), 200, "Blob not found: {:?}", check_response);

    // Fetch the blob and verify its content
    let get_response = client.get(format!("http://localhost:{}/v2/test/blobs/{}", port, expected_digest))
        .send()
        .await
        .unwrap();

    assert_eq!(get_response.status().as_u16(), 200, "Failed to get blob: {:?}", get_response);

    let downloaded_content = get_response.bytes().await.unwrap();
    assert_eq!(downloaded_content.len(), content_bytes.len(), "Downloaded content size should match original");
    assert_eq!(downloaded_content, content_bytes, "Downloaded content should match original");

    println!("Chunked upload test completed successfully!");

    // Shutdown the server
    server.abort();
}
