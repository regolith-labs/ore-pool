use std::fmt::Debug;

use ore_api::state::proof_pda;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use steel::{AccountDeserialize, Instruction, Pubkey};

use crate::error::Error;

pub async fn pda<T: AccountDeserialize + Debug>(
    rpc_client: &RpcClient,
    keypair: &Keypair,
    pool_address: &Pubkey,
    create_ix: Instruction,
) -> Result<(), Error> {
    // Submit create if pool or reservation account does not exist
    let (proof_address, _) = proof_pda(*pool_address);
    let (reservation_address, _) = ore_boost_api::state::reservation_pda(proof_address);
    let pool_data = rpc_client.get_account_data(&pool_address).await;
    let reservation_data = rpc_client.get_account_data(&reservation_address).await;
    if pool_data.is_err() || reservation_data.is_err() {
        let mut tx = Transaction::new_with_payer(&[create_ix.clone()], Some(&keypair.pubkey()));
        let hash = rpc_client.get_latest_blockhash().await?;
        tx.sign(&[keypair], hash);
        let sig = rpc_client.send_transaction(&tx).await?;
        println!("{:?}", sig);
        println!("sleeping for 10 seconds to allow rpc to catch up");
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }

    // Print account
    println!("fetching account");
    let pool_data = rpc_client.get_account_data(pool_address).await?;
    let account: &T = AccountDeserialize::try_from_bytes(pool_data.as_slice())?;
    println!("{:?}", account);
    Ok(())
}
