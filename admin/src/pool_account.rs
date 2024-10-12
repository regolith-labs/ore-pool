use ore_pool_api::state::Pool;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{signature::Keypair, signer::Signer};
use steel::AccountDeserialize;

use crate::error::Error;

pub async fn pool_account(rpc_client: &RpcClient, keypair: &Keypair) -> Result<(), Error> {
    let (pool_pda, _) = ore_pool_api::state::pool_pda(keypair.pubkey());
    println!("pool address: {:?}", pool_pda);
    let pool = rpc_client.get_account_data(&pool_pda).await?;
    let pool = Pool::try_from_bytes(pool.as_slice())?;
    println!("pool: {:?}", pool);
    Ok(())
}
