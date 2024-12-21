mod machined;
mod devprop;
mod process;
mod config;

use miette::IntoDiagnostic;
use tonic::{Request, Response, Status};
use tonic::codegen::tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Server;
use tracing::info;
use tracing_subscriber;
use machined::machine_service_server::MachineService;
use crate::config::load_config;
use crate::machined::{ClaimRequest, ClaimResponse, InstallConfig, InstallProgress};
use crate::machined::machine_service_server::MachineServiceServer;
use passwords::PasswordGenerator;


#[derive(Debug, Default)]
struct Svc {
    claim_key: String,
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

    let pg = PasswordGenerator::new()
        .length(8)
        .lowercase_letters(true)
        .uppercase_letters(true)
        .exclude_similar_characters(true)
        .spaces(false)
        .numbers(true)
        .symbols(false)
        .strict(false);

    let claim_key = pg.generate_one().into_diagnostic()?;
    info!("claim this installer with the key {}", &claim_key);

    // Now we listen for requests to claim the server from a
    info!("starting server on {}", &cfg.listen);
    let addr = cfg.listen.parse().into_diagnostic()?;
    let machined = Svc {
        claim_key
    };

    Server::builder()
        .add_service(MachineServiceServer::new(machined))
        .serve(addr)
        .await
        .into_diagnostic()?;

    Ok(())
}
