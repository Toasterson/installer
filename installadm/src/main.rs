use crate::machined::claim_request::ClaimSecret;
use crate::machined::machine_service_client::MachineServiceClient;
use crate::machined::{ClaimRequest, InstallConfig};
use crate::state::{read_state_file, save_state, Server};
use clap::{Parser, Subcommand};
use miette::Diagnostic;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;
use tonic::codec::CompressionEncoding;
use tonic::codegen::http;
use tonic::codegen::tokio_stream::StreamExt;
use tonic::transport::Channel;
use tonic::Status;
use url::Url;

mod config;
mod machined;
mod state;
mod usb;

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
    #[error(transparent)]
    JSONError(#[from] serde_json::Error),
    #[error(transparent)]
    UrlParse(#[from] url::ParseError),
    #[error("No such server please claim it first")]
    NoSuchServer,
    #[error("No parent dir")]
    NoParentDir,
    #[error("Please provide a servername none can be inferred")]
    ServerNameCannotBeInferred,
    #[error("Failed to create FAT32 partition: {0}")]
    PartitionError(String),
    #[error("Failed to download file: {0}")]
    DownloadError(#[from] reqwest::Error),
    #[error("Failed to download file: {0}")]
    DownloadCodeError(String),
    #[error("Failed to extract archive: {0}")]
    ArchiveError(String),
    #[error("Failed to find required tool: {0}")]
    ToolNotFound(String),
    #[error("Failed to execute command: {0}")]
    CommandError(String),
    #[error("Failed to download OCI image: {0}")]
    OciError(String),
    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Claim {
        url: String,
        secret: Option<String>,
        name: Option<String>,
    },
    Install {
        name: String,
        #[arg(short, long)]
        config: PathBuf,
    },
    /// Create a bootable USB stick with EFI boot files
    /// 
    /// The boot files URL is configured in one of the following locations:
    /// - /etc/installadm/config
    /// - /etc/installadm/config.<RUN_MODE>
    /// - ~/.config/installadm/config
    /// - Environment variable INSTALLADM_BOOT_FILES_URL
    CreateBootableUsb {
        /// Path to the USB device (e.g., /dev/sdb on Linux, disk2 on macOS)
        device: String,

        /// Optional OCI image to download to the USB stick
        #[arg(short, long)]
        oci_image: Option<String>,

        /// Optional size of the FAT32 partition in GB (default: 4)
        #[arg(short, long, default_value = "4")]
        size: u64,

        /// Optional additional assets URL
        #[arg(short, long)]
        assets_url: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let mut state = read_state_file()?;

    match args.command {
        Commands::Claim { secret, url, name } => {
            let url_url: Url = url.parse()?;
            if secret.is_none() {
                return Err(Error::CurrentlyPasswordClaimRequired);
            }

            let server_name = if let Some(arg_name) = name {
                arg_name
            } else if let Some(host_name_str) = url_url.host_str() {
                host_name_str.to_owned()
            } else {
                return Err(Error::ServerNameCannotBeInferred);
            };

            let claim_request = tonic::Request::new(ClaimRequest {
                claim_secret: secret.map(|s| ClaimSecret::ClaimPassword(s)),
            });
            let mut client = connect(url.as_str()).await?;

            let response = client.claim(claim_request).await?;
            let claim_response = response.into_inner();
            let srv = Server {
                name: server_name.clone(),
                uri: url,
                claim_token: claim_response.claim_token,
            };
            state.add_server(srv);
            save_state(state)?;
        }
        Commands::Install { config, name } => {
            let server = state.get_server(&name).ok_or(Error::NoSuchServer)?;
            let machineconfig = read_to_string(&config)?;
            let install_request = tonic::Request::new(InstallConfig { machineconfig });
            let mut client = connect(server.uri.as_str()).await?;
            let response = client.install(install_request).await?;
            let mut stream = response.into_inner();
            while let Some(stream_resp) = stream.next().await {
                let progress = stream_resp?;
                println!("{}: {:?}", progress.level, progress.message);
            }
        }
        Commands::CreateBootableUsb { 
            device, 
            oci_image, 
            size, 
            assets_url 
        } => {
            println!("Creating bootable USB stick on device: {}", device);
            usb::create_bootable_usb(
                &device,
                oci_image.as_deref(),
                size,
                assets_url.as_deref()
            ).await?;
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
