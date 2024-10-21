use std::str::FromStr;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair, signer::EncodableKey,
};

mod error;
mod get_or_create;
mod init;
mod member_account;
mod open_stake;
mod pool_account;
mod proof_account;

#[tokio::main]
async fn main() -> Result<(), error::Error> {
    // parse resources
    let command = command()?;
    let keypair = keypair()?;
    let rpc_client = rpc_client()?;
    let boost_mint = boost_mint();
    let pool_url = pool_url();
    // run
    match command.as_str() {
        "init" => init::init(&rpc_client, &keypair, pool_url).await,
        "open-stake" => open_stake::open_stake(&rpc_client, &keypair, boost_mint).await,
        "pool-account" => pool_account::pool_account(&rpc_client, &keypair).await,
        "proof-account" => proof_account::proof_account(&rpc_client, &keypair).await,
        "member-account" => member_account::member_account(&rpc_client, &keypair).await,
        _ => Err(error::Error::InvalidCommand),
    }
}

fn command() -> Result<String, error::Error> {
    std::env::var("COMMAND").map_err(From::from)
}

fn boost_mint() -> Option<Pubkey> {
    std::env::var("MINT").ok().and_then(|mint| {
        Pubkey::from_str(mint.as_str())
            .map_err(|err| {
                println!("{:?}", err);
                err
            })
            .ok()
    })
}

fn rpc_client() -> Result<RpcClient, error::Error> {
    std::env::var("RPC_URL")
        .map(|url| RpcClient::new_with_commitment(url, CommitmentConfig::confirmed()))
        .map_err(From::from)
}

fn keypair() -> Result<Keypair, error::Error> {
    let keypair_path = std::env::var("KEYPAIR_PATH")?;
    let keypair = Keypair::read_from_file(keypair_path.clone())
        .map_err(|_| error::Error::KeypairRead(keypair_path))?;
    Ok(keypair)
}

fn pool_url() -> Option<String> {
    std::env::var("POOL_URL").ok()
}
