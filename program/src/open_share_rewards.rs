use ore_pool_api::consts::SHARE_REWARDS;
use ore_pool_api::instruction::OpenShareRewards;
use ore_pool_api::state::{Pool, ShareRewards};
use steel::*;
use steel::{AccountInfo, ProgramError, ProgramResult};

pub fn process_open_share_rewards(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = OpenShareRewards::try_from_bytes(data)?;
    // Load accounts.
    let [signer_info, pool_info, mint_info, share_rewards_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    pool_info
        .to_account::<Pool>(&ore_pool_api::ID)?
        .check(|p| p.authority == *signer_info.key)?;
    share_rewards_info.is_empty()?.is_writable()?.has_seeds(
        &[
            SHARE_REWARDS,
            pool_info.key.as_ref(),
            mint_info.key.as_ref(),
        ],
        args.bump,
        &ore_pool_api::ID,
    )?;
    system_program.is_program(&system_program::ID)?;

    // Create the share rewards pda.
    create_account::<ShareRewards>(
        share_rewards_info,
        &ore_pool_api::ID,
        &[
            SHARE_REWARDS,
            pool_info.key.as_ref(),
            mint_info.key.as_ref(),
            &[args.bump],
        ],
        system_program,
        signer_info,
    )?;

    // Initialize share rewards account data.
    let share_rewards = share_rewards_info.to_account_mut::<ShareRewards>(&ore_pool_api::ID)?;
    share_rewards.pool = *pool_info.key;
    share_rewards.mint = *mint_info.key;

    Ok(())
}
