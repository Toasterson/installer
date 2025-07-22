mod machined;
mod platform;

mod config;
mod devprop;
mod error;
mod sysinfo;
mod process;
mod util;

use crate::config::{load_config, MachinedConfig};
use crate::machined::claim_request::ClaimSecret;
use crate::machined::install_progress;
use crate::machined::machine_service_server::MachineServiceServer;
use crate::machined::{ClaimRequest, ClaimResponse, InstallConfig, InstallProgress, SystemInfoRequest, SystemInfoResponse};
use base64::Engine;
use jwt_simple::prelude::*;
use machineconfig::MachineConfig;
use machined::machine_service_server::MachineService;
use miette::IntoDiagnostic;
use std::fs::{self, OpenOptions};
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tonic::codec::CompressionEncoding;
use tonic::codegen::tokio_stream::wrappers::ReceiverStream;
use tonic::codegen::tokio_stream::Stream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};
use tracing::{info, warn};
use tracing_subscriber::{self, prelude::*, Registry};

type ProgressMessage = Result<InstallProgress, Status>;
type ResponseStream = Pin<Box<dyn Stream<Item = ProgressMessage> + Send>>;

#[derive(Debug, Default)]
struct Svc {
    config: Arc<MachinedConfig>,
    private_key_bytes: Arc<Vec<u8>>,
}

#[tonic::async_trait]
impl MachineService for Svc {
    async fn claim(
        &self,
        request: Request<ClaimRequest>,
    ) -> Result<Response<ClaimResponse>, Status> {
        // first we check the password with the local configuration
        let claim_request = request.into_inner();
        if let Some(secret) = claim_request.claim_secret {
            match secret {
                ClaimSecret::ClaimPassword(password) => {
                    if self.config.claim_password == password {
                        let key = HS256Key::from_bytes(&self.private_key_bytes);

                        let claims = Claims::create(Duration::from_hours(2));
                        let claim_token = key
                            .authenticate(claims)
                            .map_err(|_| Status::permission_denied("wrong claims"))?;
                        Ok(Response::new(ClaimResponse { claim_token }))
                    } else {
                        Err(Status::permission_denied("wrong password"))
                    }
                }
                ClaimSecret::ClaimPayload(_) => {
                    todo!()
                }
            }
        } else {
            Err(Status::invalid_argument("Claim secret required"))
        }
    }

    type InstallStream = ResponseStream;

    async fn install(
        &self,
        request: Request<InstallConfig>,
    ) -> Result<Response<Self::InstallStream>, Status> {
        let key = HS256Key::from_bytes(&self.private_key_bytes);
        if let Some(auth_header) = request.metadata().get("Authorization") {
            let header_token_str = auth_header
                .to_str()
                .map_err(|_| Status::permission_denied("bad token"))?;
            let _ = key
                .verify_token::<NoCustomClaims>(header_token_str, None)
                .map_err(|_| Status::permission_denied("token verification failed"))?;
            let config = request.into_inner();
            let (tx, rc) = mpsc::channel(1);
            let mc: MachineConfig = knus::parse("install_config", &config.machineconfig)
                .map_err(|e| Status::invalid_argument(e.to_string()))?;
            let cfg = self.config.clone();
            tokio::spawn(async move {
                match platform::install_system(&mc, cfg.clone(), tx).await {
                    Ok(_) => {}
                    Err(_) => {}
                }
            });

            let output_stream = ReceiverStream::new(rc);
            Ok(Response::new(Box::pin(output_stream)))
        } else {
            Err(Status::not_found("Missing authorization header"))
        }
    }
    
    async fn get_system_info(
        &self,
        _request: Request<SystemInfoRequest>,
    ) -> Result<Response<SystemInfoResponse>, Status> {
        // Call the get_system_info function from the sysinfo module
        match crate::sysinfo::get_system_info() {
            Ok(system_info) => Ok(Response::new(system_info)),
            Err(status) => Err(status),
        }
    }
}

/// Check for a KDL configuration file in /usb
/// Returns the content of the file if found, None otherwise
fn check_usb_config() -> Option<(String, String)> {
    // Check for files with various extensions
    for ext in &[".kdl", ".json", ".toml", ".yaml", ".yml"] {
        let file_path = format!("/usb/machined{}", ext);
        let path = Path::new(&file_path);
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => {
                    return Some((file_path, content));
                }
                Err(e) => {
                    warn!("Failed to read {}: {}", file_path, e);
                }
            }
        }
    }
    None
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> miette::Result<()> {
    // install global collector configured based on RUST_LOG env var.
    let msg_log = OpenOptions::new()
        .append(true)
        .create(false)
        .open("/dev/msglog")
        .into_diagnostic()?;
    let subscriber = Registry::default()
        .with(
            // msglog layer, to view everything in the console as smf service have their output rerouted to a logfile by smf
            tracing_subscriber::fmt::layer()
                .compact()
                .with_writer(msg_log)
                .with_ansi(true)
                .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG),
        )
        .with(
            // stdout layer, to view everything in the console
            tracing_subscriber::fmt::layer()
                .compact()
                .with_ansi(true)
                .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG),
        );
    tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;

    let cfg = load_config()?;

    // Check for a KDL configuration file in /usb
    if let Some((file_path, config_content)) = check_usb_config() {
        info!("Found configuration file: {}", file_path);
        info!("Running installation based on USB configuration");

        // Parse the configuration
        let mc: MachineConfig = match machineconfig::parse_config(&file_path, &config_content) {
            Ok(config) => config,
            Err(e) => {
                warn!("Failed to parse configuration file: {}", e);
                return Err(e.into());
            }
        };

        // Create a channel for progress messages
        let (tx, mut rx) = mpsc::channel::<Result<InstallProgress, Status>>(100);

        // Spawn a task to handle progress messages
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Ok(progress) => {
                        let level = match progress.level {
                            0 => "DEBUG",
                            1 => "INFO",
                            2 => "WARNING",
                            3 => "ERROR",
                            _ => "UNKNOWN",
                        };

                        if let Some(message) = progress.message {
                            match message {
                                install_progress::Message::Info(info) => {
                                    info!("[{}] {}", level, info);
                                }
                                install_progress::Message::Error(error) => {
                                    warn!("[{}] {}", level, error);
                                }
                            }
                        } else {
                            info!("[{}] <no message>", level);
                        }
                    }
                    Err(status) => {
                        warn!("Error: {}", status);
                    }
                }
            }
        });

        // Run the installation
        match platform::install_system(&mc, Arc::new(cfg), tx).await {
            Ok(_) => {
                info!("Installation completed successfully");
            }
            Err(e) => {
                warn!("Installation failed: {}", e);
                return Err(miette::miette!("Installation failed: {}", e));
            }
        }

        return Ok(());
    }

    // If no configuration file was found, start the gRPC server
    // first we get all ip addresses and display them together with the claim key
    info!("No USB configuration file found, starting gRPC server");
    info!("Listing all interfaces and ip addresses available");
    let addrs = nix::ifaddrs::getifaddrs().unwrap();
    for ifaddr in addrs {
        match ifaddr.address {
            Some(address) => {
                if !ifaddr.interface_name.starts_with("lo") {
                    info!("interface {} address {}", ifaddr.interface_name, address);
                }
            }
            _ => {}
        }
    }

    info!(
        "claim this installer with the password {}",
        &cfg.claim_password
    );
    let key = if let Some(claim_secret) = &cfg.claim_key {
        let key = base64::prelude::BASE64_STANDARD
            .decode(&claim_secret.private_key)
            .into_diagnostic()?;
        key
    } else {
        HS256Key::generate().to_bytes()
    };
    // Now we listen for requests to claim the server from a
    info!("starting server on {}", &cfg.listen);
    let addr = cfg.listen.parse().into_diagnostic()?;
    let machined = Svc {
        config: Arc::new(cfg),
        private_key_bytes: Arc::new(key),
    };

    Server::builder()
        .add_service(
            MachineServiceServer::new(machined)
                .send_compressed(CompressionEncoding::Zstd)
                .accept_compressed(CompressionEncoding::Zstd),
        )
        .serve(addr)
        .await
        .into_diagnostic()?;

    Ok(())
}
