use ore_boost_api::state::Stake;
use ore_pool_api::state::ShareRewards;
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
    // get or create stake account
    let (pool_pda, _) = ore_pool_api::state::pool_pda(pubkey);
    let (stake_pda, _) = ore_pool_api::state::pool_stake_pda(pool_pda, mint);
    let open_stake_ix = ore_pool_api::sdk::open_stake(pubkey, mint);
    println!("stake pda: {:?}", stake_pda);
    get_or_create::pda::<Stake>(rpc_client, keypair, &stake_pda, open_stake_ix).await?;
    // get or create share rewards account
    let (pool_pda, _) = ore_pool_api::state::pool_pda(pubkey);
    let (share_rewards_pda, _) = ore_pool_api::state::pool_share_rewards_pda(pool_pda, mint);
    let open_share_rewards_ix = ore_pool_api::sdk::open_share_rewards(pubkey, pool_pda, mint);
    get_or_create::pda::<ShareRewards>(
        rpc_client,
        keypair,
        &share_rewards_pda,
        open_share_rewards_ix,
    )
    .await?;
    Ok(())
}
