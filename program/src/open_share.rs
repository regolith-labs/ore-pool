use ore_boost_api::state::Boost;
use ore_pool_api::prelude::*;
use steel::*;

/// Opens a new share account for pool member to deposit stake.
pub fn process_open_share(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = OpenShare::try_from_bytes(data)?;

    // Load accounts.
    let [signer_info, boost_info, mint_info, pool_info, share_info, stake_info, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    boost_info
        .to_account::<Boost>(&ore_boost_api::ID)?
        .check(|b| b.mint == *mint_info.key)?;
    mint_info.to_mint()?;
    pool_info.to_account::<Pool>(&ore_pool_api::ID)?;
    share_info.is_empty()?.is_writable()?.has_seeds(
        &[
            SHARE,
            signer_info.key.as_ref(),
            pool_info.key.as_ref(),
            mint_info.key.as_ref(),
        ],
        args.share_bump,
        &ore_pool_api::ID,
    )?;
    stake_info
        .to_account::<ore_boost_api::state::Stake>(&ore_boost_api::ID)?
        .check(|s| s.authority == *pool_info.key)?
        .check(|s| s.boost == *boost_info.key)?;
    system_program.is_program(&system_program::ID)?;

    // Create the share pda.
    create_account::<Share>(
        share_info,
        &ore_pool_api::id(),
        &[
            SHARE,
            signer_info.key.as_ref(),
            pool_info.key.as_ref(),
            mint_info.key.as_ref(),
            &[args.share_bump],
        ],
        system_program,
        signer_info,
    )?;

    // Initialize share account data.
    let share = share_info.to_account_mut::<Share>(&ore_pool_api::ID)?;
    share.authority = *signer_info.key;
    share.balance = 0;
    share.pool = *pool_info.key;
    share.mint = *mint_info.key;

    Ok(())
}
