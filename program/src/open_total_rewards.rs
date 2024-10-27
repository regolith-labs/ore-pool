use ore_pool_api::{
    consts::TOTAL_REWARDS,
    state::{Pool, TotalRewards},
};
use steel::*;

pub fn process_open_total_rewards(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer_info, pool_info, total_rewards_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    pool_info
        .as_account::<Pool>(&ore_pool_api::ID)?
        .assert(|p| p.authority == *signer_info.key)?;
    total_rewards_info
        .is_empty()?
        .is_writable()?
        .has_seeds(&[TOTAL_REWARDS, pool_info.key.as_ref()], &ore_pool_api::ID)?;
    system_program.is_program(&system_program::ID)?;

    // Create the total rewards pda.
    create_account::<TotalRewards>(
        total_rewards_info,
        system_program,
        signer_info,
        &ore_pool_api::ID,
        &[TOTAL_REWARDS, pool_info.key.as_ref()],
    )?;

    // Initialize total rewards account data.
    let total_rewards = total_rewards_info.as_account_mut::<TotalRewards>(&ore_pool_api::ID)?;
    total_rewards.pool = *pool_info.key;

    Ok(())
}
