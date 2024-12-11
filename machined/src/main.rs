mod machined;

use miette::IntoDiagnostic;
use tonic::{Request, Response, Status};
use tonic::transport::Server;
use tracing::info;
use tracing_subscriber;
use machined::machine_service_server::MachineService;
use crate::machined::{InfoRequest, InfoResponse};
use crate::machined::machine_service_server::MachineServiceServer;

#[derive(Debug, Default)]
struct Svc;

#[tonic::async_trait]
impl MachineService for Svc {
    async fn info(&self, _request: Request<InfoRequest>) -> Result<Response<InfoResponse>, Status> {
        Ok(Response::new(InfoResponse::default()))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> miette::Result<()> {
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    let addr = "[::1]:50051";
    info!("starting server on {}", addr);

    let addr = addr.parse().into_diagnostic()?;
    let machined = Svc::default();

    Server::builder()
        .add_service(MachineServiceServer::new(machined))
        .serve(addr)
        .await
        .into_diagnostic()?;

    Ok(())
}
