use ore_pool_api::{
    instruction::IncrementTotalRewards,
    state::{Pool, TotalRewards},
};
use steel::*;

pub fn process_increment_total_rewards(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = IncrementTotalRewards::try_from_bytes(data)?;
    let miner_rewards = u64::from_le_bytes(args.miner_rewards);
    let staker_rewards = u64::from_le_bytes(args.staker_rewards);
    let operator_rewards = u64::from_le_bytes(args.operator_rewards);

    // Load accounts.
    let [signer_info, pool_info, total_rewards_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    pool_info
        .to_account::<Pool>(&ore_pool_api::ID)?
        .check(|p| p.authority == *signer_info.key)?;
    let total_rewards = total_rewards_info
        .is_writable()?
        .to_account_mut::<TotalRewards>(&ore_pool_api::ID)?
        .check_mut(|tr| tr.pool == *pool_info.key)?;

    // Update rewards.
    total_rewards.miner_rewards = miner_rewards;
    total_rewards.staker_rewards = staker_rewards;
    total_rewards.operator_rewards = operator_rewards;

    Ok(())
}
