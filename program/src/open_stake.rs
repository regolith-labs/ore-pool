use ore_boost_api::loaders::load_boost;
use ore_pool_api::{consts::*, loaders::load_pool, state::Pool};
use ore_utils::*;
use solana_program::{
    self, account_info::AccountInfo, entrypoint::ProgramResult, program::invoke_signed,
    program_error::ProgramError, system_program,
};

/// Opens a new stake account for the pool in the boost program.
pub fn process_open_stake(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer, boost_info, mint_info, pool_info, pool_tokens_info, stake_info, system_program, token_program, associated_token_program, ore_boost_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_signer(signer)?;
    load_boost(boost_info, mint_info.key, false)?;
    load_any_mint(mint_info, false)?;
    load_pool(pool_info, signer.key, true)?;
    load_any(pool_tokens_info, true)?;
    load_any(stake_info, true)?;
    load_program(system_program, system_program::id())?;
    load_program(token_program, spl_token::id())?;
    load_program(associated_token_program, spl_associated_token_account::id())?;
    load_program(ore_boost_program, ore_boost_api::id())?;

    // Load pool bump for signing CPIs.
    let pool_data = pool_info.data.borrow();
    let pool = Pool::try_from_bytes(&pool_data)?;
    let pool_bump = pool.bump as u8;
    drop(pool_data);

    // Open the stake account.
    invoke_signed(
        &ore_boost_api::sdk::open(*pool_info.key, *mint_info.key),
        &[
            pool_info.clone(),
            boost_info.clone(),
            mint_info.clone(),
            stake_info.clone(),
            system_program.clone(),
        ],
        &[&[POOL, signer.key.as_ref(), &[pool_bump]]],
    )?;

    // Create token account for pending pool stake, if necessary
    if pool_tokens_info.data.borrow().is_empty() {
        create_ata(
            signer,
            pool_info,
            pool_tokens_info,
            mint_info,
            system_program,
            token_program,
            associated_token_program,
        )?;
    } else {
        load_associated_token_account(pool_tokens_info, pool_info.key, mint_info.key, true)?;
    }

    Ok(())
}
