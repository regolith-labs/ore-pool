use ore_pool_api::{
    consts::TOTAL_REWARDS,
    instruction::OpenTotalRewards,
    state::{Pool, TotalRewards},
};
use steel::*;

pub fn process_open_total_rewards(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = OpenTotalRewards::try_from_bytes(data)?;
    // Load accounts.
    let [signer_info, pool_info, total_rewards_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    pool_info
        .to_account::<Pool>(&ore_pool_api::ID)?
        .check(|p| p.authority == *signer_info.key)?;
    total_rewards_info.is_empty()?.is_writable()?.has_seeds(
        &[TOTAL_REWARDS, pool_info.key.as_ref()],
        args.bump,
        &ore_pool_api::ID,
    )?;
    system_program.is_program(&system_program::ID)?;

    // Create the total rewards pda.
    create_account::<TotalRewards>(
        total_rewards_info,
        &ore_pool_api::ID,
        &[TOTAL_REWARDS, pool_info.key.as_ref(), &[args.bump]],
        system_program,
        signer_info,
    )?;

    // Initialize total rewards account data.
    let total_rewards = total_rewards_info.to_account_mut::<TotalRewards>(&ore_pool_api::ID)?;
    total_rewards.pool = *pool_info.key;

    Ok(())
}
