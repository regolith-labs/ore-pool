use ore_api::state::proof_pda;
use ore_pool_api::state::pool_pda;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::transaction::Transaction;
use solana_sdk::{signature::Keypair, signer::Signer};

use crate::error::Error;

pub async fn init(
    rpc_client: &RpcClient,
    keypair: &Keypair,
    pool_url: Option<String>,
) -> Result<(), Error> {
    // parse arguments
    let pool_url = pool_url.ok_or(Error::MissingPoolUrl)?;
    let pool_authority = keypair.pubkey();

    // Submit create if pool or reservation account does not exist
    let (pool_address, _) = pool_pda(pool_authority);
    let (proof_address, _) = proof_pda(pool_address);
    let (reservation_address, _) = ore_boost_api::state::reservation_pda(proof_address);
    let pool_data = rpc_client.get_account_data(&pool_address).await;
    let reservation_data = rpc_client.get_account_data(&reservation_address).await;
    println!("pool address: {:?}", pool_address);
    println!("reservation address: {:?}", reservation_address);
    if pool_data.is_err() || reservation_data.is_err() {
        let cu_budget = ComputeBudgetInstruction::set_compute_unit_limit(1_000_000);
        let cu_price = ComputeBudgetInstruction::set_compute_unit_price(1_000_000);
        let launch_ix = ore_pool_api::sdk::launch(pool_authority, pool_authority, pool_url)?;
        let mut tx = Transaction::new_with_payer(&[cu_budget, cu_price, launch_ix], Some(&keypair.pubkey()));
        let hash = rpc_client.get_latest_blockhash().await?;
        tx.sign(&[keypair], hash);
        let sig = rpc_client.send_transaction(&tx).await?;
        println!("OK: {:?}", sig);
    }

    // get or create member account
    let (member_address, _) = ore_pool_api::state::member_pda(pool_authority, pool_address);
    let member_data = rpc_client.get_account_data(&member_address).await;
    println!("member address: {:?}", member_address);
    if member_data.is_err() {
        let cu_budget = ComputeBudgetInstruction::set_compute_unit_limit(1_000_000);
        let cu_price = ComputeBudgetInstruction::set_compute_unit_price(1_000_000);
        let join_ix = ore_pool_api::sdk::join(pool_authority, pool_address, pool_authority);
        let mut tx = Transaction::new_with_payer(&[cu_budget, cu_price, join_ix], Some(&keypair.pubkey()));
        let hash = rpc_client.get_latest_blockhash().await?;
        tx.sign(&[keypair], hash);
        let sig = rpc_client.send_transaction(&tx).await?;
        println!("OK: {:?}", sig);
    }
    
    Ok(())
}
