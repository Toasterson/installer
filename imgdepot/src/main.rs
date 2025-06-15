mod api;
mod config;
mod error;
mod storage;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    Router,
    routing::get,
    response::IntoResponse,
};
use opentelemetry::{global, KeyValue};
use opentelemetry::metrics::{MeterProvider, Unit};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    trace::{self, Sampler},
    Resource,
};
use prometheus::{Encoder, TextEncoder};
use tokio::net::TcpListener;
use tokio::signal;
use tracing::{info, warn, instrument};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry,
};

use crate::api::routes;
use crate::config::AppConfig;
use crate::storage::Storage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize OpenTelemetry tracing
    let tracer = init_tracer()?;

    // Initialize tracing subscriber with OpenTelemetry
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    // Load configuration
    let config = AppConfig::load()?;
    info!("Loaded configuration: {:?}", config);

    // Initialize storage
    let storage = Storage::new(&config).await?;
    let storage = Arc::new(storage);

    // Initialize OpenTelemetry metrics with Prometheus
    let registry = prometheus::Registry::new();
    let exporter = opentelemetry_prometheus::exporter().with_registry(registry.clone())
        .build()?;
    let meter_provider = opentelemetry_sdk::metrics::MeterProvider::builder()
        .with_resource(Resource::new(vec![
            KeyValue::new("service.name", "imgdepot"),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        ]))
        .with_reader(exporter)
        .build();
    let meter = meter_provider.meter("imgdepot");

    // Create counters and other metrics
    let request_counter = meter
        .u64_counter("http_requests_total")
        .with_description("Total number of HTTP requests")
        .with_unit(Unit::new("requests"))
        .init();

    let blob_size_histogram = meter
        .f64_histogram("blob_size_bytes")
        .with_description("Size of blobs in bytes")
        .with_unit(Unit::new("bytes"))
        .init();

    // Create AppMetrics struct with metrics
    let app_metrics = Arc::new(routes::AppMetrics {
        request_counter,
        blob_size_histogram,
    });

    // Create application state
    let app_state = (Arc::clone(&storage), Arc::clone(&app_metrics));

    // Create a clone of the registry for the metrics endpoint
    let metrics_registry = registry.clone();

    if let Some(tracer) = tracer {
        Registry::default()
            .with(env_filter)
            .with(fmt::layer().with_target(true))
            .with(OpenTelemetryLayer::new(tracer))
            .init();
    } else {
        Registry::default()
            .with(env_filter)
            .with(fmt::layer().with_target(true))
            .init();
    }
    
    // Build application with metrics endpoint
    let app = Router::new()
        .route("/metrics", get(move || metrics_handler(metrics_registry.clone())))
        .merge(routes::registry_router(app_state))
        .with_state((storage, app_metrics));

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("Starting imgdepotd server on {}", addr);
    info!("Metrics available at http://{}:{}/metrics", addr.ip(), addr.port());

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Shut down OpenTelemetry tracer provider
    global::shutdown_tracer_provider();

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, starting graceful shutdown");
}

// Initialize OpenTelemetry tracer
fn init_tracer() -> anyhow::Result<Option<trace::Tracer>> {
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();

    if let Some(otlp_endpoint) = otlp_endpoint {
        info!("OpenTelemetry OTLP endpoint: {}", otlp_endpoint);
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(otlp_endpoint)
            )
            .with_trace_config(
                trace::config()
                    .with_sampler(Sampler::AlwaysOn)
                    .with_resource(Resource::new(vec![
                        KeyValue::new("service.name", "imgdepot"),
                        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                    ]))
            )
            .install_batch(opentelemetry_sdk::runtime::Tokio)?;

        Ok(Some(tracer))
    } else {
        Ok(None)
    }
}

// This function has been removed as it was not working correctly

// Metrics endpoint handler for Prometheus scraping
#[instrument(name = "metrics_handler", skip_all)]
async fn metrics_handler(registry: prometheus::Registry) -> impl IntoResponse {
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();

    info!("Serving metrics");

    // Encode metrics to the buffer
    if let Err(e) = encoder.encode(&registry.gather(), &mut buffer) {
        warn!("Failed to encode metrics: {}", e);
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to encode metrics").into_response();
    }

    // Convert the buffer to a string
    match String::from_utf8(buffer) {
        Ok(metrics_string) => metrics_string.into_response(),
        Err(e) => {
            warn!("Failed to convert metrics to string: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to convert metrics to string").into_response()
        }
    }
}
