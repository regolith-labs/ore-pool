use std::fmt::Debug;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use steel::{AccountDeserialize, Instruction, Pubkey};

use crate::error::Error;

pub async fn pda<T: AccountDeserialize + Debug>(
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
