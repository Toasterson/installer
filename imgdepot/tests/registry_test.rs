use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time::sleep;

use opentelemetry::metrics::MeterProvider;

use oci_util::digest::OciDigest;
use oci_util::distribution::client::Registry;
use oci_util::models::ImageManifest;

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
async fn test_api_version_check() {
    // Start the test server
    let (server, port) = start_test_server().await;

    // Make a request to the API version endpoint
    let response = reqwest::get(format!("http://localhost:{}/v2/", port))
        .await
        .unwrap();

    // Check that the response is successful
    assert_eq!(response.status().as_u16(), 200);

    // Shutdown the server
    server.abort();
}

#[tokio::test]
async fn test_manifest_operations() {
    // Start the test server
    let (server, port) = start_test_server().await;

    // Create a registry client
    let registry = Registry::new(
        format!("http://localhost:{}", port),
        None, // No auth for testing
    );

    // Create a session
    let mut session = registry.new_session("test".to_string());

    // Create a simple manifest
    let manifest = ImageManifest {
        schema_version: 2,
        media_type: "application/vnd.oci.image.manifest.v1+json".to_string(),
        config: oci_util::models::Descriptor {
            media_type: "application/vnd.oci.image.config.v1+json".to_string(),
            digest: OciDigest::from_str("sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855").unwrap(),
            size: 0
        },
        layers: vec![]
    };

    // Push the manifest
    let result = session.register_manifest("latest", &manifest).await;
    assert!(result.is_ok());

    // Query the manifest
    let result = session.query_manifest("latest").await;
    assert!(result.is_ok());
    let manifest_result = result.unwrap();
    assert!(manifest_result.is_some());

    // Shutdown the server
    server.abort();
}

#[tokio::test]
async fn test_blob_operations() {
    // Start the test server
    let (server, port) = start_test_server().await;

    // Create a registry client
    let registry = Registry::new(
        format!("http://localhost:{}", port),
        None, // No auth for testing
    );

    // Create a session
    let mut session = registry.new_session("test".to_string());

    // Create test content
    let content = b"test blob content".to_vec();

    // Upload the blob
    let result = session.upload_content(
        None, // No progress tracking
        "application/octet-stream".to_string(),
        content.clone().as_slice(),
    ).await;
    assert!(result.is_ok());
    let descriptor = result.unwrap();

    // Check if the blob exists
    let exists = session.exists_digest(&descriptor.digest).await.unwrap();
    assert!(exists);

    // Fetch the blob
    let response = session.fetch_blob(&descriptor.digest).await.unwrap();
    assert_eq!(response.status().as_u16(), 200);

    let blob_content = response.bytes().await.unwrap();
    assert_eq!(blob_content.to_vec(), content);

    // Shutdown the server
    server.abort();
}

#[tokio::test]
async fn test_repository_listing() {
    // Start the test server
    let (server, port) = start_test_server().await;

    // Create a registry client
    let registry = Registry::new(
        format!("http://localhost:{}", port),
        None, // No auth for testing
    );

    // Create a session and push a manifest to create a repository
    let mut session = registry.new_session("test-repo".to_string());

    // Create a simple manifest
    let manifest = ImageManifest {
        schema_version: 2,
        media_type: "application/vnd.oci.image.manifest.v1+json".to_string(),
        config: oci_util::models::Descriptor {
            media_type: "application/vnd.oci.image.config.v1+json".to_string(),
            digest: OciDigest::from_str("sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855").unwrap(),
            size: 0
        },
        layers: vec![]
    };

    // Push the manifest to create the repository
    let result = session.register_manifest("latest", &manifest).await;
    assert!(result.is_ok());

    // List repositories
    let response = reqwest::get(format!("http://localhost:{}/v2/_catalog", port))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);

    let catalog: serde_json::Value = response.json().await.unwrap();
    let repositories = catalog["repositories"].as_array().unwrap();

    // Check that our test repository is in the list
    let found = repositories.iter().any(|repo| repo.as_str().unwrap() == "test-repo");
    assert!(found);

    // Shutdown the server
    server.abort();
}
