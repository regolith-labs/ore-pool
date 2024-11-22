use ore_pool_api::consts::NONCE;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use crate::error::Error;

pub async fn nonce_account(rpc_client: &RpcClient, keypair: &Keypair) -> Result<(), Error> {
    // derive pubkey with seed for nonce account
    let authority = keypair.pubkey();
    let nonce_pubkey = ore_pool_api::state::pool_nonce_account(keypair.pubkey())?;
    // get or create
    let nonce_account = rpc_client.get_account(&nonce_pubkey).await;
    match nonce_account {
        Ok(nonce_account) => {
            let nonce_account_state =
                solana_client::nonce_utils::state_from_account(&nonce_account)?;
            match nonce_account_state {
                solana_program::nonce::State::Uninitialized => {
                    println!("account data found but unitialized");
                }
                solana_sdk::nonce::State::Initialized(data) => {
                    println!("address: {:?}", nonce_pubkey);
                    println!("{:?}", data);
                }
            }
        }
        Err(_) => {
            // create the account with the derived seed
            let nonce_rent = rpc_client
                .get_minimum_balance_for_rent_exemption(solana_program::nonce::State::size())
                .await?;
            let create_nonce_account_ixs =
                solana_program::system_instruction::create_nonce_account_with_seed(
                    &authority,    // payer of the transaction
                    &nonce_pubkey, // derived address
                    &authority,    // base pubkey
                    NONCE,         // seed
                    &authority,    // authority to advance the nonce
                    nonce_rent,    // lamports to allocate
                );
            // sign and submit transaction
            let mut tx =
                Transaction::new_with_payer(create_nonce_account_ixs.as_slice(), Some(&authority));
            let blockhash = rpc_client.get_latest_blockhash().await?;
            tx.sign(&[keypair], blockhash);
            let sig = rpc_client.send_transaction(&tx).await?;
            println!("sig: {:?}", sig);
        }
    }
    Ok(())
}
