use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use crate::error::Error;

pub async fn init(
    rpc_client: &RpcClient,
    keypair: &Keypair,
    pool_url: String,
) -> Result<(), Error> {
    let pubkey = keypair.pubkey();
    let ix = ore_pool_api::sdk::launch(pubkey, pubkey, pool_url)?;
    let hash = rpc_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[ix], Some(&pubkey));
    tx.sign(&[keypair], hash);
    let sig = rpc_client.send_transaction(&tx).await?;
    println!("{:?}", sig);
    Ok(())
}
