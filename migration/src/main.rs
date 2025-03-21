mod client;
mod error;

use std::{str::FromStr, sync::Arc, time::Duration};

use ore_pool_api::state::{Member, Pool};
use solana_sdk::signer::Signer;
use steel::Pubkey;
use tokio::time::sleep;

use crate::client::{AsyncClient, Client};

#[tokio::main]
pub async fn main() {
    let client = client::Client::new()?;
    let client = std::sync::Arc::new(client);

    let pools = client.get_pools().await?;

    // Phase 1: Initialize migration
    for pool in pools {
        println!("Pool: {:?}", pool.0);
        let ix = ore_pool_api::sdk::migrate_pool(signer.pubkey(), pool.0);
        let cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);
        let cu_price_ix = ComputeBudgetInstruction::set_compute_unit_price(10_000);
        let final_ixs = &[cu_limit_ix, cu_price_ix, ix];
        let hash = rpc.get_latest_blockhash().await.unwrap();
        let mut tx = Transaction::new_with_payer(final_ixs.as_slice(), Some(&signer.pubkey()));
        tx.sign(&[&signer], hash);
        println!("{:?}", tx);
        // rpc.send_transaction(&tx).await.unwrap();
    }

    // TODO: Phase 2: Migrate member balances
    for pool in pools {
        // Fetch pool
        let pool_account = Pool::try_from_bytes(&pool.1.data).unwrap();
        println!("Pool: {}", pool.0);

        // Fetch members of the given pool
        let member_filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            Member::discriminator().to_le_bytes().to_vec(),
        ));
        // let pool_member_filter =
        //     RpcFilterType::Memcmp(Memcmp::new_raw_bytes(16, pool.0.to_bytes().to_vec()));
        let members = client.get_pool_members(&pool.0).await?;

        // Migrate each member balance
        for member in members {
            let ix = ore_pool_api::sdk::migrate_member_balance(signer.pubkey(), pool.0, member.0);
            let cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);
            let cu_price_ix = ComputeBudgetInstruction::set_compute_unit_price(10_000);
            let final_ixs = &[cu_limit_ix, cu_price_ix, ix];
            let hash = rpc.get_latest_blockhash().await.unwrap();
            let mut tx = Transaction::new_with_payer(final_ixs.as_slice(), Some(&signer.pubkey()));
            tx.sign(&[&signer], hash);
            println!("Migrate member balance: {:?}", tx);
            // rpc.send_transaction(&tx).await.unwrap();
        }
    }
}

