use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, signer::EncodableKey};

mod error;
mod init;

#[tokio::main]
async fn main() -> Result<(), error::Error> {
    // parse resources
    let keypair = keypair()?;
    let pool_url = pool_url()?;
    let rpc_client = rpc_client()?;
    let command = command()?;
    // run
    match command.as_str() {
        "init" => init::init(&rpc_client, &keypair, pool_url).await,
        _ => Err(error::Error::InvalidCommand),
    }
}

fn command() -> Result<String, error::Error> {
    std::env::var("COMMAND").map_err(From::from)
}

fn rpc_url() -> Result<String, error::Error> {
    std::env::var("RPC_URL").map_err(From::from)
}

fn rpc_client() -> Result<RpcClient, error::Error> {
    let rpc_url = rpc_url()?;
    Ok(RpcClient::new_with_commitment(
        rpc_url,
        CommitmentConfig::confirmed(),
    ))
}

fn keypair() -> Result<Keypair, error::Error> {
    let keypair_path = keypair_path()?;
    let keypair = Keypair::read_from_file(keypair_path.clone())
        .map_err(|_| error::Error::KeypairRead(keypair_path))?;
    Ok(keypair)
}

fn keypair_path() -> Result<String, error::Error> {
    std::env::var("KEYPAIR_PATH").map_err(From::from)
}

fn pool_url() -> Result<String, error::Error> {
    std::env::var("POOL_URL").map_err(From::from)
}
