use std::fs::read_to_string;
use std::path::PathBuf;
use std::str::FromStr;
use clap::{Parser, Subcommand};
use miette::Diagnostic;
use thiserror::Error;
use tonic::codec::CompressionEncoding;
use tonic::codegen::http;
use tonic::codegen::tokio_stream::StreamExt;
use tonic::Status;
use tonic::transport::Channel;
use crate::machined::claim_request::ClaimSecret;
use crate::machined::{ClaimRequest, InstallConfig};
use crate::machined::machine_service_client::MachineServiceClient;
use crate::state::{read_state_file, save_state, Server};

mod machined;
mod state;

#[derive(Debug, Error, Diagnostic)]
pub enum Error {
    #[error("server responded with error: {0}")]
    ServerError(#[from] Status),
    #[error(transparent)]
    TransportError(#[from] tonic::transport::Error),
    #[error(transparent)]
    URLConversionError(#[from] http::uri::InvalidUri),
    #[error("No App Directory")]
    NoAppDir,
    #[error("Currently only password claims are supported")]
    CurrentlyPasswordClaimRequired,
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("No such server please claim it first")]
    NoSuchServer,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Parser)]
struct Args {
    #[arg(global = true)]
    server_name: String,
    #[command(subcommand)]
    command: Commands
}

#[derive(Debug, Subcommand)]
enum Commands {
    Claim {
        secret: Option<String>,
        url: String,
    },
    Install {
        #[arg(short, long)]
        config: PathBuf,
    }
}


#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let mut state = read_state_file()?;

    match args.command {
        Commands::Claim { secret, url } => {
            if secret.is_none() {
                return Err(Error::CurrentlyPasswordClaimRequired);
            }

            let claim_request = tonic::Request::new(ClaimRequest{
                claim_secret: secret.map(|s| ClaimSecret::ClaimPassword(s)),
            });
            let mut client = connect(url.as_str()).await?;

            let response = client.claim(claim_request).await?;
            let claim_response = response.into_inner();
            let srv =  Server{
                name: args.server_name,
                uri: url,
                claim_token: claim_response.claim_token,
            };
            state.add_server(srv);
            save_state(state)?;
        }
        Commands::Install { config } => {
            let server = state.get_server(&args.server_name).ok_or(Error::NoSuchServer)?;
            let machineconfig = read_to_string(&config)?;
            let install_request =  tonic::Request::new(InstallConfig{
                machineconfig,
            });
            let mut client = connect(server.uri.as_str()).await?;
            let response = client.install(install_request).await?;
            let mut stream = response.into_inner();
            while let Some(stream_resp) = stream.next().await {
                let progress = stream_resp?;
                println!("{}: {:?}", progress.level, progress.message);
            }
        }
    }

    Ok(())
}

async fn connect(url: &str) -> Result<MachineServiceClient<Channel>> {
    let channel = Channel::builder(http::Uri::from_str(url)?)
        .connect()
        .await?;

    let client = MachineServiceClient::new(channel)
        .send_compressed(CompressionEncoding::Zstd)
        .accept_compressed(CompressionEncoding::Zstd);
    Ok(client)
}