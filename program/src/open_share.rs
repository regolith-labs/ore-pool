use std::mem::size_of;

use ore_boost_api::loaders::{load_boost, load_stake};
use ore_pool_api::{consts::*, instruction::OpenShare, loaders::load_pool, state::Share};
use ore_utils::*;
use solana_program::{
    self, account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    system_program,
};

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
    load_signer(signer)?;
    load_boost(boost_info, mint_info.key, false)?;
    load_any_mint(mint_info, false)?;
    load_pool(pool_info, signer.key, true)?;
    load_uninitialized_pda(
        share_info,
        &[
            SHARE,
            signer.key.as_ref(),
            pool_info.key.as_ref(),
            mint_info.key.as_ref(),
        ],
        args.share_bump,
        &ore_pool_api::id(),
    )?;
    load_stake(stake_info, pool_info.key, boost_info.key, false)?;
    load_program(system_program, system_program::id())?;

    // Create the share pda.
    create_pda(
        share_info,
        &ore_pool_api::id(),
        8 + size_of::<Share>(),
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
