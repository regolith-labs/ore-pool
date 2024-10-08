use ore_boost_api::state::{Boost, Stake};
use ore_pool_api::{consts::*, instruction::OpenShare, loaders::load_any_pool, state::Share};
use steel::*;

/// Opens a new share account for pool member to deposit stake.
pub fn process_open_share(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = OpenShare::try_from_bytes(data)?;

    // Load accounts.
    let [signer, boost_info, mint_info, pool_info, share_info, stake_info, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer.is_signer()?;
    boost_info
        .is_writable()?
        .to_account::<Boost>(&ore_boost_api::ID)?
        .check(|b| b.mint == *mint_info.key)?;
    mint_info.to_mint()?;
    load_any_pool(pool_info, false)?;
    share_info.is_empty()?.is_writable()?.has_seeds(
        &[
            SHARE,
            signer.key.as_ref(),
            pool_info.key.as_ref(),
            mint_info.key.as_ref(),
        ],
        args.share_bump,
        &ore_pool_api::ID,
    )?;
    stake_info
        .to_account::<Stake>(&ore_boost_api::ID)?
        .check(|s| s.authority == *pool_info.key)?
        .check(|s| s.boost == *boost_info.key)?;
    system_program.is_program(&system_program::ID)?;

    // Create the share pda.
    create_account::<Share>(
        share_info,
        &ore_pool_api::id(),
        &[
            SHARE,
            signer.key.as_ref(),
            pool_info.key.as_ref(),
            mint_info.key.as_ref(),
            &[args.share_bump],
        ],
        system_program,
        signer,
    )?;

    // Initialize share account data.
    let mut share_data = share_info.try_borrow_mut_data()?;
    share_data[0] = Share::discriminator();
    let share = Share::try_from_bytes_mut(&mut share_data)?;
    share.authority = *signer.key;
    share.balance = 0;
    share.pool = *pool_info.key;
    share.mint = *mint_info.key;

    Ok(())
}
