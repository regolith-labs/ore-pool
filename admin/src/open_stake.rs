use ore_boost_api::state::Stake;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

use crate::{error::Error, get_or_create};

pub async fn open_stake(
    rpc_client: &RpcClient,
    keypair: &Keypair,
    mint: Option<Pubkey>,
) -> Result<(), Error> {
    let mint = mint.ok_or(Error::MissingBoostMint)?;
    let pubkey = keypair.pubkey();
    let (pool_pda, _) = ore_pool_api::state::pool_pda(pubkey);
    let (stake_pda, _) = ore_pool_api::state::pool_stake_pda(pool_pda, mint);
    let ix = ore_pool_api::sdk::open_stake(pubkey, mint);
    get_or_create::pda::<Stake>(rpc_client, keypair, &stake_pda, ix).await?;
    Ok(())
}
