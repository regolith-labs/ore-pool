use ore_boost_api::state::Boost;
use ore_pool_api::prelude::*;
use steel::*;

/// Opens a new share account for pool member to deposit stake.
pub fn process_open_share(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer_info, boost_info, mint_info, pool_info, share_info, stake_info, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    boost_info
        .as_account::<Boost>(&ore_boost_api::ID)?
        .assert(|b| b.mint == *mint_info.key)?;
    mint_info.as_mint()?;
    pool_info.as_account::<Pool>(&ore_pool_api::ID)?;
    share_info.is_empty()?.is_writable()?.has_seeds(
        &[
            SHARE,
            signer_info.key.as_ref(),
            pool_info.key.as_ref(),
            mint_info.key.as_ref(),
        ],
        &ore_pool_api::ID,
    )?;
    stake_info
        .as_account::<ore_boost_api::state::Stake>(&ore_boost_api::ID)?
        .assert(|s| s.authority == *pool_info.key)?
        .assert(|s| s.boost == *boost_info.key)?;
    system_program.is_program(&system_program::ID)?;

    // Create the share pda.
    create_account::<Share>(
        share_info,
        system_program,
        signer_info,
        &ore_pool_api::id(),
        &[
            SHARE,
            signer_info.key.as_ref(),
            pool_info.key.as_ref(),
            mint_info.key.as_ref(),
        ],
    )?;

    // Initialize share account data.
    let share = share_info.as_account_mut::<Share>(&ore_pool_api::ID)?;
    share.authority = *signer_info.key;
    share.balance = 0;
    share.pool = *pool_info.key;
    share.mint = *mint_info.key;
    share.last_withdrawal = Clock::get()?.unix_timestamp as u64;

    Ok(())
}
