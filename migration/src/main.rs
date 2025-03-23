mod client;
mod error;

use std::{str::FromStr, sync::Arc, time::Duration};

use ore_api::state::proof_pda;
use solana_sdk::signer::Signer;
use steel::Pubkey;
use tokio::time::sleep;

use crate::client::{AsyncClient, Client};

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let client = client::Client::new()?;
    let client = std::sync::Arc::new(client);

    // Phase 1: Initialize migration
    let pools = client.rpc.get_pools().await?;
    println!("Total pools: {}\n", pools.len());
    for (pool_address, pool) in pools.clone() {
        // Skip other pools
        // if pool_address != Pubkey::from_str("9tQd8NkKUx1J3UJFtZHgBCKjk4fvWhwFCDh882MKgQiP")? {
        //     continue;
        // }

        println!("Pool: {:?}", pool_address);
        println!("  {:?} members", pool.total_members);
        println!("  {:?} submissions", pool.total_submissions);
        // migrate_pool(client.clone(), pool_address).await?;
        verify_pool_total_rewards(client.clone(), pool_address).await?;
    }

    // Phase 2: Migrate member balances
    for (pool_address, pool) in pools.clone() {
        // Skip other pools
        if pool_address != Pubkey::from_str("9tQd8NkKUx1J3UJFtZHgBCKjk4fvWhwFCDh882MKgQiP")? {
            continue;
        }

        // Fetch migration
        // let migration = client.clone().rpc.get_migration(&pool_address).await?;
        println!("Pool: {:?}", pool_address);
        println!("  {:?} members", pool.total_members);
        println!("  {:?} submissions", pool.total_submissions);
        // println!(
        //     "  {} of {} migrated",
        //     migration.members_migrated, pool.total_members
        // );

        // Fetch members of the given pool
        let mut members = client.clone().rpc.get_pool_members(&pool_address).await?;
        members.sort_by_key(|(_, m)| m.id);

        // Migrate each member balance
        for (member_address, member) in members {
            // if member.id <= migration.members_migrated {
            //     println!("Skipping...");
            //     continue;
            // }
            // println!(
            //     "[{}/{}] {} {}",
            //     member.id, pool.total_members, member.authority, member.total_balance
            // );
            // sleep(Duration::from_secs(1)).await;
            // migrate_member_balance(client.clone(), pool_address, member_address).await?;
        }
    }

    // Phase 3: Verify migration
    for (pool_address, pool) in pools {
        verify_pool_is_migrated(client.clone(), pool_address).await?;
    }

    Ok(())
}

async fn migrate_pool(client: Arc<Client>, pool_address: Pubkey) -> anyhow::Result<()> {
    let ix = ore_pool_api::sdk::migrate_pool(client.keypair.pubkey(), pool_address);
    match client.send_transaction(&[ix]).await {
        Ok(sig) => println!("    OK: https://solscan.io/tx/{}", sig),
        Err(e) => println!("    FAIL: {}", e),
    }
    Ok(())
}

async fn migrate_member_balance(
    client: Arc<Client>,
    pool_address: Pubkey,
    member_address: Pubkey,
) -> anyhow::Result<()> {
    let ix = ore_pool_api::sdk::migrate_member_balance(
        client.keypair.pubkey(),
        pool_address,
        member_address,
    );
    match client.send_transaction(&[ix]).await {
        Ok(sig) => println!("    OK: https://solscan.io/tx/{}", sig),
        Err(e) => println!("    FAIL: {}", e),
    }
    Ok(())
}

async fn verify_pool_total_rewards(
    client: Arc<Client>,
    pool_address: Pubkey,
) -> anyhow::Result<()> {
    let proof_address = proof_pda(pool_address).0;
    let proof = client.clone().rpc.get_proof(&proof_address).await?;
    let pool = client.clone().rpc.get_pool(&pool_address).await?;
    let members = client.clone().rpc.get_pool_members(&pool_address).await?;
    let mut net_total_rewards = 0;
    for member in members {
        net_total_rewards += member.1.balance;
    }
    // assert_eq!(net_total_rewards, pool.total_rewards);
    // assert!(net_total_rewards <= proof.total_rewards);
    println!("  {} {}", net_total_rewards, proof.balance);
    if net_total_rewards > proof.balance {
        println!("  INVALID POOL!");
        println!("  Difference: {}\n", net_total_rewards - proof.balance);
    } else {
        println!("  Pool rewards are valid.\n");
    }

    Ok(())
}

async fn verify_pool_is_migrated(client: Arc<Client>, pool_address: Pubkey) -> anyhow::Result<()> {
    let pool = client.clone().rpc.get_pool(&pool_address).await?;
    let members = client.clone().rpc.get_pool_members(&pool_address).await?;
    let mut expected_total_rewards = 0;
    for member in members {
        expected_total_rewards += member.1.balance;
    }
    assert_eq!(expected_total_rewards, pool.total_rewards);
    Ok(())
}
