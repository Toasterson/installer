mod machined;
mod devprop;
mod process;
mod config;

use std::sync::Arc;
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


#[derive(Debug, Default)]
struct Svc {
    config: Arc<MachinedConfig>,
}

#[tonic::async_trait]
impl MachineService for Svc {
    async fn claim(&self, request: Request<ClaimRequest>) -> Result<Response<ClaimResponse>, Status> {
        todo!()
    }

    type InstallStream = ReceiverStream<Result<InstallProgress, Status>>;

    async fn install(&self, request: Request<InstallConfig>) -> Result<Response<Self::InstallStream>, Status> {
        todo!()
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

    // Now we listen for requests to claim the server from a
    info!("starting server on {}", &cfg.listen);
    let addr = cfg.listen.parse().into_diagnostic()?;
    let machined = Svc {
        config: Arc::new(cfg),
    };

    Server::builder()
        .add_service(MachineServiceServer::new(machined))
        .serve(addr)
        .await
        .into_diagnostic()?;

    Ok(())
}
