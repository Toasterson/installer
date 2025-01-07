mod machined;
mod platform;

mod config;
mod devprop;
mod process;
mod util;

use std::pin::Pin;
use std::sync::Arc;
use std::sync::mpsc::SendError;
use base64::Engine;
use jwt_simple::prelude::*;
use machineconfig::MachineConfig;
use miette::IntoDiagnostic;
use tonic::{Request, Response, Status};
use tonic::codegen::tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Server;
use tracing::{debug, info};
use tracing_subscriber;
use machined::machine_service_server::MachineService;
use crate::config::{load_config, MachinedConfig};
use crate::machined::{ClaimRequest, ClaimResponse, InstallConfig, InstallProgress};
use crate::machined::machine_service_server::MachineServiceServer;
use passwords::PasswordGenerator;
use tokio::sync::mpsc;
use tonic::codegen::tokio_stream;
use tonic::codegen::tokio_stream::Stream;
use crate::machined::claim_request::ClaimSecret;
use crate::platform::Error;

type ResponseStream = Pin<Box<dyn Stream<Item = Result<InstallProgress, Status>> + Send>>;

#[derive(Debug, Default)]
struct Svc {
    config: Arc<MachinedConfig>,
    private_key_bytes: Arc<Vec<u8>>,
}

#[tonic::async_trait]
impl MachineService for Svc {
    async fn claim(&self, request: Request<ClaimRequest>) -> Result<Response<ClaimResponse>, Status> {
        // first we check the password with the local configuration
        let claim_request = request.into_inner();
        if let Some(secret) = claim_request.claim_secret {
            match secret {
                ClaimSecret::ClaimPassword(password) => {
                    if self.config.claim_password == password {
                        let key = HS256Key::from_bytes(&self.private_key_bytes);

                        let claims = Claims::create(Duration::from_hours(2));
                        let claim_token = key.authenticate(claims)
                            .map_err(|e| Status::permission_denied("wrong claims"))?;
                        Ok(Response::new(ClaimResponse{
                            claim_token,
                        }))
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

    async fn install(&self, request: Request<InstallConfig>) -> Result<Response<Self::InstallStream>, Status> {
        let config = request.into_inner();
        let (tx, rc) = mpsc::channel(1);
        let mc: MachineConfig = knus::parse("install_config", &config.machineconfig).map_err(|e| Status::invalid_argument(e.to_string()))?;
        tokio::spawn(async move {
            match platform::install_system(&mc, tx) {
                Ok(_) => {}
                Err(_) => {}
            }
        });

        let output_stream = ReceiverStream::new(rc);
        Ok(Response::new(Box::pin(output_stream)))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> miette::Result<()> {
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    let cfg = load_config()?;

    // first we get all ip addresses and display them together with the claim key
    info!("Listing all interfaces and ip addresses available");
    let addrs = nix::ifaddrs::getifaddrs().unwrap();
    for ifaddr in addrs {
        match ifaddr.address {
            Some(address) => {
                if !ifaddr.interface_name.starts_with("lo") {
                    info!("interface {} address {}",
                             ifaddr.interface_name, address);
                }
            },
            _ => {}
        }
    }

    info!("claim this installer with the password {}", &cfg.claim_password);
    let key = if let Some(claim_secret) = &cfg.claim_key {
        let key =
            base64::prelude::BASE64_STANDARD
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
        .add_service(MachineServiceServer::new(machined))
        .serve(addr)
        .await
        .into_diagnostic()?;

    Ok(())
}
