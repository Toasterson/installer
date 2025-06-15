use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time::sleep;

use opentelemetry::metrics::MeterProvider;

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
