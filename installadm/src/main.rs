use clap::Parser;
use tonic::codec::CompressionEncoding;
use tonic::transport::Channel;
use crate::machined::ClaimRequest;
use crate::machined::machine_service_client::MachineServiceClient;

mod machined;

#[derive(Debug, Parser)]
struct Args {

}

//TODO Commands Claim, INstall etc.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channel = Channel::builder("http://[::1]:50051".parse().unwrap())
        .connect()
        .await?;

    let mut client = MachineServiceClient::new(channel)
        .send_compressed(CompressionEncoding::Zstd)
        .accept_compressed(CompressionEncoding::Zstd);

    let claim_request = tonic::Request::new(ClaimRequest{
        claim_secret: None,
    });

    let response = client.claim(claim_request).await?;

    dbg!(response);
    Ok(())
}