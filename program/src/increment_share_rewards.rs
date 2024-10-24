use ore_pool_api::{
    instruction::IncrementShareRewards,
    state::{Pool, ShareRewards},
};
use steel::*;

pub fn process_increment_share_rewards(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = IncrementShareRewards::try_from_bytes(data)?;
    let rewards = u64::from_le_bytes(args.rewards);

    // Load accounts.
    let [signer_info, pool_info, mint_info, share_rewards_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    pool_info
        .to_account::<Pool>(&ore_pool_api::ID)?
        .check(|p| p.authority == *signer_info.key)?;
    mint_info.to_mint()?;
    let share_rewards = share_rewards_info
        .is_writable()?
        .to_account_mut::<ShareRewards>(&ore_pool_api::ID)?
        .check_mut(|sr| sr.pool == *pool_info.key)?
        .check_mut(|sr| sr.mint == *mint_info.key)?;

    // Update rewards.
    share_rewards.rewards = rewards;

    Ok(())
}
