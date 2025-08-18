use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use std::str::FromStr;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use tracing::info;
use tokio::net::TcpListener;
use utoipa::{OpenApi, ToSchema, IntoParams};

// Generated from build.rs via tonic-build
pub mod machined_grpc {
    tonic::include_proto!("machined");
}

use machined_grpc::machine_service_client::MachineServiceClient;
use machined_grpc::{claim_request, ClaimRequest, SystemInfoRequest};

#[derive(Serialize, Clone, ToSchema)]
struct DiskDto {
    device: String,
    vendor: String,
    product: String,
    serial: String,
    size_bytes: u64,
    removable: bool,
    solid_state: bool,
    paths: Vec<String>,
    fault_status: String,
    location_code: String,
    chassis_bay: String,
}

#[derive(Serialize, Clone, Default, ToSchema)]
struct PartitionDto {
    device: String,
    size_bytes: u64,
    parent_device: Option<String>,
}

#[derive(Serialize, Clone, ToSchema)]
struct StorageDto {
    disks: Vec<DiskDto>,
    partitions: Vec<PartitionDto>,
}

#[derive(Clone, Debug)]
struct MachineEntry {
    endpoint: String,
    name: Option<String>,
    token: Option<String>,
}

#[derive(Clone)]
struct AppState {
    machines: Arc<RwLock<HashMap<String, MachineEntry>>>,
    default_machine: Option<String>,
}

#[derive(Error, Debug)]
enum ApiError {
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),
    #[error("transport error: {0}")]
    Transport(#[from] tonic::transport::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("internal error: {0}")]
    Other(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            ApiError::Grpc(ref s) => {
                // Map tonic status to HTTP status roughly
                match s.code() {
                    tonic::Code::PermissionDenied => StatusCode::UNAUTHORIZED,
                    tonic::Code::InvalidArgument => StatusCode::BAD_REQUEST,
                    tonic::Code::NotFound => StatusCode::NOT_FOUND,
                    _ => StatusCode::BAD_GATEWAY,
                }
            }
            ApiError::Transport(_) => StatusCode::BAD_GATEWAY,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = serde_json::json!({ "error": self.to_string() });
        (status, Json(body)).into_response()
    }
}

#[derive(Serialize, ToSchema)]
struct HealthResp {
    status: &'static str,
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Health status", body = HealthResp)
    )
)]
async fn health() -> impl IntoResponse {
    Json(HealthResp { status: "ok" })
}

#[derive(Serialize, Clone, ToSchema)]
struct MachineDto {
    id: String,
    endpoint: String,
    name: Option<String>,
    has_token: bool,
}

#[derive(Deserialize, ToSchema)]
struct AddMachineBody {
    endpoint: String,
    name: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct AddMachineResp {
    id: String,
}

#[utoipa::path(
    get,
    path = "/api/machines",
    responses((status = 200, description = "List machines", body = [MachineDto]))
)]
async fn list_machines(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let map = state.machines.read().await;
    let list: Vec<MachineDto> = map
        .iter()
        .map(|(id, m)| MachineDto {
            id: id.clone(),
            endpoint: m.endpoint.clone(),
            name: m.name.clone(),
            has_token: m.token.is_some(),
        })
        .collect();
    Ok(Json(list))
}

#[utoipa::path(
    post,
    path = "/api/machines",
    request_body = AddMachineBody,
    responses(
        (status = 200, description = "Machine added", body = AddMachineResp),
        (status = 400, description = "Bad request")
    )
)]
async fn add_machine(
    State(state): State<AppState>,
    Json(body): Json<AddMachineBody>,
) -> Result<impl IntoResponse, ApiError> {
    if body.endpoint.trim().is_empty() {
        return Err(ApiError::BadRequest("endpoint required".into()));
    }
    // Simple ID: use endpoint as ID if unique, otherwise suffix with number
    let mut id = body
        .name
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| body.endpoint.clone());
    {
        let map = state.machines.read().await;
        if map.contains_key(&id) {
            id = format!("{}#{}", id, map.len() + 1);
        }
    }
    let mut map = state.machines.write().await;
    map.insert(
        id.clone(),
        MachineEntry {
            endpoint: body.endpoint,
            name: body.name,
            token: None,
        },
    );
    Ok(Json(AddMachineResp { id }))
}

#[derive(Deserialize, ToSchema)]
struct ClaimBody {
    claim_password: Option<String>,
    claim_payload: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct ClaimResp {
    claim_token: String,
}

#[utoipa::path(
    post,
    path = "/api/machines/{id}/claim",
    params(
        ("id" = String, Path, description = "Machine ID")
    ),
    request_body = ClaimBody,
    responses((status = 200, description = "Claim token", body = ClaimResp))
)]
async fn claim_machine(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ClaimBody>,
) -> Result<impl IntoResponse, ApiError> {
    let (endpoint, req) = {
        let map = state.machines.read().await;
        let m = map
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("machine '{}' not found", id)))?;
        let req = if let Some(pw) = body.claim_password.clone() {
            ClaimRequest {
                claim_secret: Some(claim_request::ClaimSecret::ClaimPassword(pw)),
            }
        } else if let Some(payload) = body.claim_payload.clone() {
            ClaimRequest {
                claim_secret: Some(claim_request::ClaimSecret::ClaimPayload(payload)),
            }
        } else {
            return Err(ApiError::BadRequest(
                "claim_password or claim_payload required".into(),
            ));
        };
        (m.endpoint.clone(), req)
    };

    let mut client = MachineServiceClient::connect(endpoint).await?;
    client = client.accept_compressed(tonic::codec::CompressionEncoding::Zstd);
    let resp = client.claim(req).await?.into_inner();

    // Store token
    {
        let mut map = state.machines.write().await;
        if let Some(m) = map.get_mut(&id) {
            m.token = Some(resp.claim_token.clone());
        }
    }

    Ok(Json(ClaimResp {
        claim_token: resp.claim_token,
    }))
}

#[derive(Deserialize, Default, IntoParams, ToSchema)]
struct SystemInfoQuery {
    /// Optional JWT claim token; if omitted, uses stored token if available
    token: Option<String>,
}

#[derive(Deserialize, Default, IntoParams, ToSchema)]
struct StorageQuery {
    /// Optional JWT claim token; if omitted, uses stored token if available
    token: Option<String>,
    /// Whether to include partitions (true/1/yes/on) in the response
    include_partitions: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/machines/{id}/system-info",
    params(
        ("id" = String, Path, description = "Machine ID"),
        SystemInfoQuery
    ),
    responses((status = 200, description = "Protobuf bytes of SystemInfoResponse"))
)]
async fn system_info_machine(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<SystemInfoQuery>,
) -> Result<impl IntoResponse, ApiError> {
    use axum::http::header;
    use prost::Message;

    let (endpoint, stored_token) = {
        let map = state.machines.read().await;
        let m = map
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("machine '{}' not found", id)))?;
        (m.endpoint.clone(), m.token.clone())
    };

    let mut client = MachineServiceClient::connect(endpoint).await?;
    client = client.accept_compressed(tonic::codec::CompressionEncoding::Zstd);

    let mut req = tonic::Request::new(SystemInfoRequest {});
    if let Some(token) = q.token.or(stored_token) {
        let mv = tonic::metadata::MetadataValue::from_str(&token)
            .map_err(|e| ApiError::Other(e.to_string()))?;
        req.metadata_mut().insert("Authorization", mv);
    }
    let resp = client.get_system_info(req).await?.into_inner();

    let mut buf = Vec::new();
    resp.encode(&mut buf)
        .map_err(|e| ApiError::Other(e.to_string()))?;
    Ok(([(header::CONTENT_TYPE, "application/octet-stream")], buf))
}

// Backward-compatible single-machine routes
async fn claim_route(
    State(state): State<AppState>,
    Json(body): Json<ClaimBody>,
) -> Result<impl IntoResponse, ApiError> {
    let machine_id = {
        let map = state.machines.read().await;
        if map.len() == 1 {
            map.keys().next().cloned()
        } else {
            None
        }
    };
    let id = machine_id
        .or_else(|| state.default_machine.clone())
        .ok_or_else(|| ApiError::BadRequest("no default machine; use /api/machines/{id}/claim".into()))?;
    claim_machine(State(state), Path(id), Json(body)).await
}

async fn system_info_route(
    State(state): State<AppState>,
    Query(q): Query<SystemInfoQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let machine_id = {
        let map = state.machines.read().await;
        if map.len() == 1 {
            map.keys().next().cloned()
        } else {
            None
        }
    };
    let id = machine_id
        .or_else(|| state.default_machine.clone())
        .ok_or_else(|| ApiError::BadRequest("no default machine; use /api/machines/{id}/system-info".into()))?;
    system_info_machine(State(state), Path(id), Query(q)).await
}

#[utoipa::path(
    get,
    path = "/api/machines/{id}/storage",
    params(
        ("id" = String, Path, description = "Machine ID"),
        StorageQuery
    ),
    responses((status = 200, description = "Storage devices", body = StorageDto))
)]
async fn storage_machine(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<StorageQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (endpoint, stored_token) = {
        let map = state.machines.read().await;
        let m = map
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("machine '{}' not found", id)))?;
        (m.endpoint.clone(), m.token.clone())
    };

    let mut client = MachineServiceClient::connect(endpoint).await?;
    client = client.accept_compressed(tonic::codec::CompressionEncoding::Zstd);

    let mut req = tonic::Request::new(SystemInfoRequest {});
    if let Some(token) = q.token.or(stored_token) {
        let mv = tonic::metadata::MetadataValue::from_str(&token)
            .map_err(|e| ApiError::Other(e.to_string()))?;
        req.metadata_mut().insert("Authorization", mv);
    }
    let resp = client.get_system_info(req).await?.into_inner();

    let disks: Vec<DiskDto> = resp
        .disks
        .into_iter()
        .map(|d| DiskDto {
            device: d.device,
            vendor: d.vendor,
            product: d.product,
            serial: d.serial,
            size_bytes: d.size_bytes,
            removable: d.removable,
            solid_state: d.solid_state,
            paths: d.paths,
            fault_status: d.fault_status,
            location_code: d.location_code,
            chassis_bay: d.chassis_bay,
        })
        .collect();

    let include_parts = match q.include_partitions.as_deref() {
        Some("1") | Some("true") | Some("yes") | Some("on") => true,
        _ => false,
    };

    let partitions: Vec<PartitionDto> = if include_parts {
        resp.partitions
            .into_iter()
            .map(|p| PartitionDto {
                device: p.device,
                size_bytes: p.size_bytes,
                parent_device: if p.parent_device.is_empty() { None } else { Some(p.parent_device) },
            })
            .collect()
    } else {
        Vec::new()
    };

    Ok(Json(StorageDto { disks, partitions }))
}

#[derive(Deserialize, ToSchema)]
struct GenerateConfigBody {
    filename: Option<String>,
    content: String,
}

#[derive(Serialize, ToSchema)]
struct GenerateConfigResp {
    image: String,
    boot_environment_name: Option<String>,
    pools: usize,
    hostname: Option<String>,
}

#[derive(Deserialize, Debug, ToSchema)]
struct VdevInput {
    #[serde(rename = "type")]
    vtype: String,
    #[serde(default)]
    devices: Vec<String>,
}

#[derive(Deserialize, Debug, ToSchema)]
struct PoolInput {
    name: String,
    #[serde(default)]
    vdevs: Vec<VdevInput>,
}

#[derive(Deserialize, Debug, ToSchema)]
struct GeneratePoolConfigReq {
    pool: PoolInput,
}

#[derive(Serialize, Debug, ToSchema)]
struct GeneratePoolConfigResp {
    kdl: String,
    warnings: Vec<String>,
}

fn escape_kdl(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[utoipa::path(
    post,
    path = "/api/pool/config",
    request_body = GeneratePoolConfigReq,
    responses((status = 200, description = "Generated KDL and warnings", body = GeneratePoolConfigResp))
)]
async fn generate_pool_config(
    Json(body): Json<GeneratePoolConfigReq>,
) -> Result<impl IntoResponse, ApiError> {
    let mut warnings: Vec<String> = Vec::new();
    let pool = body.pool;

    let mut kdl = String::new();
    let name = escape_kdl(&pool.name);
    kdl.push_str(&format!("pool \"{}\" {{\n", name));

    for v in pool.vdevs {
        let mut vtype = v.vtype.to_lowercase();
        if vtype == "stripe" {
            if v.devices.len() <= 1 {
                warnings.push("type 'stripe' with <=1 device mapped to 'mirror'".to_string());
                vtype = "mirror".to_string();
            } else {
                warnings.push("type 'stripe' mapped to 'raidz' for compatibility".to_string());
                vtype = "raidz".to_string();
            }
        }
        kdl.push_str(&format!("  vdev \"{}\" {{\n", escape_kdl(&vtype)));
        if !v.devices.is_empty() {
            kdl.push_str("    disks");
            for d in v.devices.into_iter() {
                let d_esc = escape_kdl(&d);
                kdl.push_str(&format!(" \"{}\"", d_esc));
            }
            kdl.push('\n');
        }
        kdl.push_str("  }\n");
    }

    kdl.push_str("}\n");

    Ok(Json(GeneratePoolConfigResp { kdl, warnings }))
}

#[utoipa::path(
    post,
    path = "/api/generate-config",
    request_body = GenerateConfigBody,
    responses((status = 200, description = "Summary of parsed config", body = GenerateConfigResp))
)]
async fn generate_config_route(
    Json(body): Json<GenerateConfigBody>,
) -> Result<impl IntoResponse, ApiError> {
    let path = body
        .filename
        .unwrap_or_else(|| "stdin.kdl".to_string());
    let mc = machineconfig::parse_config(&path, &body.content)
        .map_err(|e| ApiError::Other(e.to_string()))?;

    let hostname = mc.sysconfig.hostname.clone();
    Ok(Json(GenerateConfigResp {
        image: mc.image,
        boot_environment_name: mc.boot_environment_name,
        pools: mc.pools.len(),
        hostname: if hostname.is_empty() {
            None
        } else {
            Some(hostname)
        },
    }))
}

async fn index_handler() -> impl IntoResponse {
    Html("<html><head><title>Installer UI</title></head><body><h1>Installer UI</h1><p>Use the API endpoints: <code>/api/machines</code>, <code>/api/machines/{id}/claim</code>, <code>/api/machines/{id}/system-info</code>, <code>/api/generate-config</code>.</p></body></html>")
}

#[derive(OpenApi)]
#[openapi(
    info(title = "Installer UI API", version = "0.1.0"),
    paths(
        health,
        list_machines,
        add_machine,
        claim_machine,
        system_info_machine,
        storage_machine,
        generate_config_route,
        generate_pool_config
    ),
    components(
        schemas(
            HealthResp,
            MachineDto,
            AddMachineBody,
            AddMachineResp,
            ClaimBody,
            ClaimResp,
            DiskDto,
            PartitionDto,
            StorageDto,
            SystemInfoQuery,
            StorageQuery,
            GenerateConfigBody,
            GenerateConfigResp,
            VdevInput,
            PoolInput,
            GeneratePoolConfigReq,
            GeneratePoolConfigResp
        )
    )
)]
struct ApiDoc;

async fn openapi_json() -> impl IntoResponse {
    Json(ApiDoc::openapi())
}

fn initial_machines() -> (HashMap<String, MachineEntry>, Option<String>) {
    // Start with an empty registry; machines are added via the API/UI at runtime.
    (HashMap::new(), None)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let bind: SocketAddr = std::env::var("UI_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8080".into())
        .parse()?;

    let (initial, default_machine) = initial_machines();
    let state = AppState {
        machines: Arc::new(RwLock::new(initial)),
        default_machine,
    };

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/health", get(health))
        .route("/api/openapi.json", get(openapi_json))
        .route("/api/generate-config", post(generate_config_route))
        .route("/api/pool/config", post(generate_pool_config))
        // Multi-machine management
        .route("/api/machines", get(list_machines).post(add_machine))
        .route("/api/machines/{id}/claim", post(claim_machine))
        .route("/api/machines/{id}/system-info", get(system_info_machine))
        .route("/api/machines/{id}/storage", get(storage_machine))
        // Backward-compatible single-machine endpoints
        .route("/api/claim", post(claim_route))
        .route("/api/system-info", get(system_info_route))
        .with_state(state)
        .nest_service("/static", ServeDir::new("static"));

    let listener = TcpListener::bind(bind).await?;
    info!("installer-ui listening on {}", bind);
    axum::serve(listener, app).await?;
    Ok(())
}
