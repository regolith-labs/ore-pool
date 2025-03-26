use ore_pool_api::state::Member;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{signature::Keypair, signer::Signer};
use steel::AccountDeserialize;

use crate::error::Error;

/// the member account is of interest
/// because this is where the operator commissions will be attributed.
/// this command will fetch and print the address and decoded data of the member account.
/// to manage this account (claim, stake, etc), use the ore-cli.
pub async fn member_account(rpc_client: &RpcClient, keypair: &Keypair) -> Result<(), Error> {
    let (pool_pda, _) = ore_pool_api::state::pool_pda(keypair.pubkey());
    let (member_pda, _) = ore_pool_api::state::member_pda(keypair.pubkey(), pool_pda);
    println!("member address: {:?}", member_pda);
    let data = rpc_client.get_account_data(&member_pda).await?;
    let member = Member::try_from_bytes(data.as_slice())?;
    println!("{:?}", member);
    Ok(())
}

pub async fn member_account_lookup(
    rpc_client: &RpcClient,
    keypair: &Keypair,
    pubkey: Result<Pubkey, Error>,
) -> Result<(), Error> {
    let pubkey = pubkey?;
    let (pool_pda, _) = ore_pool_api::state::pool_pda(keypair.pubkey());
    let (member_pda, _) = ore_pool_api::state::member_pda(pubkey, pool_pda);
    println!("member address: {:?}", member_pda);
    let data = rpc_client.get_account_data(&member_pda).await?;
    let member = Member::try_from_bytes(data.as_slice())?;
    println!("{:?}", member);
    Ok(())
}
