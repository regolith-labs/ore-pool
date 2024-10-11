use std::fmt::Debug;

use ore_pool_api::state::{Member, Pool};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use steel::{AccountDeserialize, Instruction};

use crate::error::Error;

pub async fn init(
    rpc_client: &RpcClient,
    keypair: &Keypair,
    pool_url: Option<String>,
) -> Result<(), Error> {
    // parse arguments
    let pool_url = pool_url.ok_or(Error::MissingPoolUrl)?;
    let pubkey = keypair.pubkey();
    // get or create pool account
    let (pool_pda, _) = ore_pool_api::state::pool_pda(pubkey);
    let launch_ix = ore_pool_api::sdk::launch(pubkey, pubkey, pool_url)?;
    println!("pool address: {:?}", pool_pda);
    get_or_create_pda::<Pool>(rpc_client, keypair, &pool_pda, launch_ix).await?;
    // get or create member account
    let (member_pda, _) = ore_pool_api::state::member_pda(pubkey, pool_pda);
    let join_ix = ore_pool_api::sdk::join(pubkey, pool_pda, pubkey);
    println!("member address: {:?}", member_pda);
    get_or_create_pda::<Member>(rpc_client, keypair, &member_pda, join_ix).await?;
    Ok(())
}

async fn get_or_create_pda<T: AccountDeserialize + Debug>(
    rpc_client: &RpcClient,
    keypair: &Keypair,
    pda: &Pubkey,
    create_ix: Instruction,
) -> Result<(), Error> {
    let data = rpc_client.get_account_data(pda).await;
    match data {
        Err(_) => {
            let mut tx = Transaction::new_with_payer(&[create_ix], Some(&keypair.pubkey()));
            let hash = rpc_client.get_latest_blockhash().await?;
            tx.sign(&[keypair], hash);
            let sig = rpc_client.send_transaction(&tx).await?;
            println!("{:?}", sig);
            println!("sleeping for 10 seconds to allow rpc to catch up");
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            println!("fetching account");
            let data = rpc_client.get_account_data(pda).await?;
            let account: &T = AccountDeserialize::try_from_bytes(data.as_slice())?;
            println!("{:?}", account);
            Ok(())
        }
        Ok(data) => {
            let account: &T = AccountDeserialize::try_from_bytes(data.as_slice())?;
            println!("{:?}", account);
            Ok(())
        }
    }
}
