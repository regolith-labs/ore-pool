use ore_pool_api::consts::NONCE;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use crate::error::Error;

pub async fn nonce_account(rpc_client: &RpcClient, keypair: &Keypair) -> Result<(), Error> {
    // derive pubkey with seed for nonce account
    let authority = keypair.pubkey();
    let nonce_account = ore_pool_api::state::pool_nonce_account(keypair.pubkey())?;
    // create the account with the derived seed
    let nonce_rent = rpc_client
        .get_minimum_balance_for_rent_exemption(solana_program::nonce::State::size())
        .await?;
    // let create_account_with_seed_ix = solana_program::system_instruction::create_account_with_seed(
    //     &authority,                                  // payer of the transaction
    //     &nonce_account,                              // derived address
    //     &authority,                                  // base pubkey
    //     NONCE,                                       // seed
    //     nonce_rent,                                  // lamports to allocate
    //     solana_program::nonce::State::size() as u64, // space to allocate
    //     &solana_program::system_program::ID,         // owner program
    // );
    let create_nonce_account_ixs =
        solana_program::system_instruction::create_nonce_account_with_seed(
            &authority,                          // payer of the transaction
            &nonce_account,                      // derived address
            &authority,                          // base pubkey
            NONCE,                               // seed
            &solana_program::system_program::ID, // owner program
            nonce_rent,                          // lamports to allocate
        );
    // sign and submit transaction
    let mut tx = Transaction::new_with_payer(create_nonce_account_ixs.as_slice(), Some(&authority));
    let blockhash = rpc_client.get_latest_blockhash().await?;
    tx.sign(&[keypair], blockhash);
    let sig = rpc_client.send_transaction(&tx).await?;
    println!("sig: {:?}", sig);
    Ok(())
}
