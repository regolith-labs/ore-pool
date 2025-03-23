mod client;
mod error;

use std::{str::FromStr, sync::Arc, time::Duration};

use ore_api::{consts::MINT_ADDRESS, state::proof_pda};
use ore_pool_api::state::migration_pda;
use solana_sdk::signer::Signer;
use steel::{token, Pubkey};
use tokio::time::sleep;

use crate::client::{AsyncClient, Client};

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let client = client::Client::new()?;
    let client = std::sync::Arc::new(client);

    // Phase 1: Initialize migration
    let pools = client.rpc.get_pools().await?;
    println!("Total pools: {}\n", pools.len());
    // for (pool_address, pool) in pools.clone() {
    //     // Skip other pools
    //     // if pool_address != Pubkey::from_str("9tQd8NkKUx1J3UJFtZHgBCKjk4fvWhwFCDh882MKgQiP")? {
    //     //     continue;
    //     // }

    //     println!("Pool: {:?}", pool_address);
    //     println!("  {:?} members", pool.total_members);
    //     println!("  {:?} submissions", pool.total_submissions);
    //     migrate_pool(client.clone(), pool_address).await?;
    //     // verify_pool_total_rewards(client.clone(), pool_address).await?;
    // }

    // panic!("Stop here");

    // Phase 2: Migrate member balances
    // for (pool_address, pool) in pools.clone() {
    //     // Skip other pools
    //     if pool_address != Pubkey::from_str("9RrEyMNFhFcrqVikWby5rVn1eXeKHr2SwGRbPhZ7wDCK")? {
    //         continue;
    //     }

    //     // migrate_quick(client.clone(), pool_address).await?;
    //     // sleep(Duration::from_secs(5)).await;
    //     verify_pool_is_migrated(client.clone(), pool_address).await?;
    //     return Ok(());

    //     // Fetch migration
    //     let migration_address = migration_pda(pool_address).0;
    //     let migration = client.clone().rpc.get_migration(&migration_address).await?;
    //     println!("Pool: {:?}", pool_address);
    //     println!("  {:?} members", pool.total_members);
    //     println!("  {:?} submissions", pool.total_submissions);
    //     println!(
    //         "  {} of {} migrated\n",
    //         migration.members_migrated, pool.total_members
    //     );

    //     // Fetch members of the given pool
    //     let mut members = client.clone().rpc.get_pool_members(&pool_address).await?;
    //     members.sort_by_key(|(_, m)| m.id);

    //     // Migrate members in batches of 10
    //     let mut batch_count = 0;
    //     let batch_size = 23;
    //     let batches = members.chunks(batch_size);
    //     for chunk in batches.clone() {
    //         // Skip batches that have already been migrated
    //         if let Some((_, member)) = chunk.last() {
    //             if member.id < migration.members_migrated {
    //                 println!("Skipping...");
    //                 batch_count += 1;
    //                 continue;
    //             }
    //         }

    //         // Process the batch
    //         batch_count += 1;
    //         println!(
    //             "  Processing batch {} of {}. {} members",
    //             batch_count,
    //             batches.len(),
    //             chunk.len()
    //         );
    //         let member_addresses: Vec<Pubkey> = chunk.iter().map(|(addr, _)| *addr).collect();
    //         migrate_member_balance_batch(client.clone(), pool_address, member_addresses).await?;
    //         sleep(Duration::from_secs(3)).await;

    //         // Break after 10 batches
    //         // if batch_count % 10 == 0 {
    //         //     return Ok(());
    //         // }
    //     }

    //     // Verify the pool is migrated
    //     sleep(Duration::from_secs(5)).await;
    //     verify_pool_is_migrated(client.clone(), pool_address).await?;
    // }

    // Phase 3: Verify migration
    for (pool_address, _pool) in pools {
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

async fn migrate_member_balance_batch(
    client: Arc<Client>,
    pool_address: Pubkey,
    member_addresses: Vec<Pubkey>,
) -> anyhow::Result<()> {
    let mut ixs = vec![];
    for member_address in member_addresses {
        let ix = ore_pool_api::sdk::migrate_member_balance(
            client.keypair.pubkey(),
            pool_address,
            member_address,
        );
        ixs.push(ix);
    }
    match client.send_transaction(&ixs).await {
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
    let proof_address = proof_pda(pool_address).0;
    let proof = client.clone().rpc.get_proof(&proof_address).await?;
    let pool_tokens_ata =
        spl_associated_token_account::get_associated_token_address(&pool_address, &MINT_ADDRESS);
    let pool_tokens_balance = if let Ok(Some(token_account)) =
        client.clone().rpc.get_token_account(&pool_tokens_ata).await
    {
        spl_token::ui_amount_to_amount(
            token_account.token_amount.ui_amount.unwrap(),
            token_account.token_amount.decimals,
        )
    } else {
        0
    };
    let mut expected_total_rewards = 0;
    for member in members {
        expected_total_rewards += member.1.balance;
    }

    println!("Pool: {:?}", pool_address);
    println!("  {:?} members", pool.total_members);
    println!("  {:?} submissions", pool.total_submissions);
    println!("  {:?} total rewards", pool.total_rewards);
    println!("  {:?} expected total rewards", expected_total_rewards);
    println!("  {:?} proof total rewards", proof.balance);
    if pool.total_rewards > proof.balance {
        println!("  {:?} DEBT", pool.total_rewards - proof.balance);
        println!("  {:?} Pool reserves", pool_tokens_balance);
        println!(
            "  Covered: {:?}",
            pool.total_rewards <= (proof.balance + pool_tokens_balance)
        );
    }
    assert_eq!(expected_total_rewards, pool.total_rewards);
    println!("  Pool is migrated!\n");
    Ok(())
}

async fn migrate_quick(client: Arc<Client>, pool_address: Pubkey) -> anyhow::Result<()> {
    let members = client.clone().rpc.get_pool_members(&pool_address).await?;
    let mut expected_total_rewards = 0;
    for member in members {
        expected_total_rewards += member.1.balance;
    }
    let ix = ore_pool_api::sdk::migrate_quick(
        client.keypair.pubkey(),
        pool_address,
        expected_total_rewards,
    );
    match client.send_transaction(&[ix]).await {
        Ok(sig) => println!("    OK: https://solscan.io/tx/{}", sig),
        Err(e) => println!("    FAIL: {}", e),
    }
    // assert_eq!(expected_total_rewards, pool.total_rewards);
    // println!("  Pool is migrated!\n");
    Ok(())
}
