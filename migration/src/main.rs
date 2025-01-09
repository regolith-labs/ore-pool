use ore_pool_api::state::{Member, Pool};
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::{
    commitment_config::CommitmentConfig, compute_budget::ComputeBudgetInstruction,
    signature::read_keypair_file, signer::Signer, transaction::Transaction,
};
use steel::{AccountDeserialize, Discriminator};

#[tokio::main]
pub async fn main() {
    // Create client
    let signer = read_keypair_file("../migration-admin-key.json").unwrap();
    let url = "https://mainnet.helius-rpc.com/?api-key=TODO";
    let rpc = RpcClient::new_with_commitment(url.to_owned(), CommitmentConfig::confirmed());

    // Fetch pools
    let pool_filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
        0,
        Pool::discriminator().to_le_bytes().to_vec(),
    ));
    let Ok(pools) = rpc
        .get_program_accounts_with_config(
            &ore_pool_api::ID,
            RpcProgramAccountsConfig {
                filters: Some(vec![pool_filter]),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    data_slice: None,
                    commitment: None,
                    min_context_slot: None,
                },
                with_context: None,
            },
        )
        .await
    else {
        return;
    };
    println!("Pools {:?}\n", pools.len());

    // TODO Phase 1: Initialize migration
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
    // for pool in pools {
    //     // Fetch members of the given pool
    //     let member_filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
    //         0,
    //         Member::discriminator().to_le_bytes().to_vec(),
    //     ));
    //     let pool_member_filter =
    //         RpcFilterType::Memcmp(Memcmp::new_raw_bytes(16, pool.0.to_bytes().to_vec()));
    //     let Ok(members) = rpc
    //         .get_program_accounts_with_config(
    //             &ore_pool_api::ID,
    //             RpcProgramAccountsConfig {
    //                 filters: Some(vec![member_filter, pool_member_filter]),
    //                 account_config: RpcAccountInfoConfig {
    //                     encoding: Some(UiAccountEncoding::Base64),
    //                     data_slice: None,
    //                     commitment: None,
    //                     min_context_slot: None,
    //                 },
    //                 with_context: None,
    //             },
    //         )
    //         .await
    //     else {
    //         return;
    //     };
    //     let pool_account = Pool::try_from_bytes(&pool.1.data).unwrap();
    //     println!("Pool: {}", pool.0);
    //     println!("Expected members: {}", pool_account.total_members);
    //     println!("Actual members: {}\n", members.len());

    //     // Migrate each member balance
    //     for member in members {
    //         let ix = ore_pool_api::sdk::migrate_member_balance(signer.pubkey(), pool.0, member.0);
    //         let cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);
    //         let cu_price_ix = ComputeBudgetInstruction::set_compute_unit_price(10_000);
    //         let final_ixs = &[cu_limit_ix, cu_price_ix, ix];
    //         let hash = rpc.get_latest_blockhash().await.unwrap();
    //         let mut tx = Transaction::new_with_payer(final_ixs.as_slice(), Some(&signer.pubkey()));
    //         tx.sign(&[&signer], hash);
    //         rpc.send_transaction(&tx).await.unwrap();
    //     }
    // }
}
