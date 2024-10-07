use ore_api::state::Proof;
use ore_utils::AccountDeserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{signature::Keypair, signer::Signer};

use crate::error::Error;

pub async fn proof_account(rpc_client: &RpcClient, keypair: &Keypair) -> Result<(), Error> {
    let (pool_pda, _) = ore_pool_api::state::pool_pda(keypair.pubkey());
    let (proof_pda, _) = ore_pool_api::state::pool_proof_pda(pool_pda);
    let proof = rpc_client.get_account_data(&proof_pda).await?;
    let proof = Proof::try_from_bytes(proof.as_slice())?;
    println!("proof address: {:?}", proof_pda);
    println!("proof: {:?}", proof);
    Ok(())
}
