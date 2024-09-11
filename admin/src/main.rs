use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::Keypair,
    signer::{EncodableKey, Signer},
    transaction::Transaction,
};

mod error;

#[tokio::main]
async fn main() -> Result<(), error::Error> {
    let keypair = keypair()?;
    let pool_url = pool_url()?;
    let rpc_client = rpc_client()?;
    // build open pool ix
    let signer = keypair.pubkey();
    let miner = keypair.pubkey();
    let ix = ore_pool_api::sdk::launch(signer, miner, pool_url)?;
    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[ix], Some(&signer));
    tx.sign(&[&keypair], blockhash);
    match rpc_client.send_transaction(&tx).await {
        Ok(sig) => {
            println!("sig: {:?}", sig);
        }
        Err(err) => {
            println!("{:?}", err);
        }
    }
    Ok(())
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
