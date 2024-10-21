use ore_pool_api::state::{Member, Pool, TotalRewards};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{signature::Keypair, signer::Signer};

use crate::{error::Error, get_or_create};

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
    get_or_create::pda::<Pool>(rpc_client, keypair, &pool_pda, launch_ix).await?;
    // get or create member account
    let (member_pda, _) = ore_pool_api::state::member_pda(pubkey, pool_pda);
    let join_ix = ore_pool_api::sdk::join(pubkey, pool_pda, pubkey);
    println!("member address: {:?}", member_pda);
    get_or_create::pda::<Member>(rpc_client, keypair, &member_pda, join_ix).await?;
    // get or create total rewards account
    let (total_rewards_pda, _) = ore_pool_api::state::pool_total_rewards(pool_pda);
    let open_total_rewards_ix = ore_pool_api::sdk::open_total_rewards(pubkey, pool_pda);
    get_or_create::pda::<TotalRewards>(
        rpc_client,
        keypair,
        &total_rewards_pda,
        open_total_rewards_ix,
    )
    .await?;
    Ok(())
}
