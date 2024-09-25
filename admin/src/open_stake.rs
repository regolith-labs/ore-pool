use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};

use crate::error::Error;

pub async fn open_stake(
    rpc_client: &RpcClient,
    keypair: &Keypair,
    mint: Option<Pubkey>,
) -> Result<(), Error> {
    let mint = mint.ok_or(Error::MissingBoostMint)?;
    let pubkey = keypair.pubkey();
    let ix = ore_pool_api::sdk::open_stake(pubkey, mint);
    let mut tx = Transaction::new_with_payer(&[ix], Some(&pubkey));
    let hash = rpc_client.get_latest_blockhash().await?;
    tx.sign(&[keypair], hash);
    let sig = rpc_client.send_transaction(&tx).await?;
    println!("{:?}", sig);
    Ok(())
}
