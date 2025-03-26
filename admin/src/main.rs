use std::str::FromStr;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, signer::EncodableKey};

mod error;
mod init;
mod member_account;
mod pool_account;
mod proof_account;

#[tokio::main]
async fn main() -> Result<(), error::Error> {
    // parse resources
    let command = command()?;
    let keypair = keypair()?;
    let rpc_client = rpc_client()?;
    let pool_url = pool_url();
    let pubkey = pubkey();
    // run
    match command.as_str() {
        "init" => init::init(&rpc_client, &keypair, pool_url).await,
        "pool-account" => pool_account::pool_account(&rpc_client, &keypair).await,
        "proof-account" => proof_account::proof_account(&rpc_client, &keypair).await,
        "member-account" => member_account::member_account(&rpc_client, &keypair).await,
        "member-account-lookup" => {
            member_account::member_account_lookup(&rpc_client, &keypair, pubkey).await
        }
        "member-account-gpa" => member_account::member_account_gpa(&rpc_client, pubkey).await,
        _ => Err(error::Error::InvalidCommand),
    }
}

fn command() -> Result<String, error::Error> {
    std::env::var("COMMAND").map_err(From::from)
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

fn pubkey() -> Result<Pubkey, error::Error> {
    let pubkey_str = std::env::var("PUBKEY")?;
    let pubkey = Pubkey::from_str(pubkey_str.as_str())?;
    Ok(pubkey)
}
