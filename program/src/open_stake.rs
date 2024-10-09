use ore_boost_api::state::Boost;
use ore_pool_api::prelude::*;
use steel::*;

/// Opens a new stake account for the pool in the boost program.
pub fn process_open_stake(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer_info, boost_info, mint_info, pool_info, pool_tokens_info, stake_info, system_program, token_program, associated_token_program, ore_boost_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    boost_info
        .to_account::<Boost>(&ore_boost_api::ID)?
        .check(|b| b.mint == *mint_info.key)?;
    mint_info.to_mint()?;
    let pool = pool_info
        .to_account_mut::<Pool>(&ore_pool_api::ID)?
        .check_mut(|p| p.authority == *signer_info.key)?;
    // load_any(pool_tokens_info, true)?;
    // load_any(stake_info, true)?;
    system_program.is_program(&system_program::ID)?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;
    ore_boost_program.is_program(&ore_boost_api::ID)?;

    // Open the stake account.
    let pool_bump = pool.bump as u8;
    solana_program::program::invoke_signed(
        &ore_boost_api::sdk::open(*pool_info.key, *signer_info.key, *mint_info.key),
        &[
            pool_info.clone(),
            signer_info.clone(),
            boost_info.clone(),
            mint_info.clone(),
            stake_info.clone(),
            system_program.clone(),
        ],
        &[&[POOL, signer_info.key.as_ref(), &[pool_bump]]],
    )?;

    // Create token account for pending pool stake, if necessary
    if pool_tokens_info.data.borrow().is_empty() {
        create_associated_token_account(
            signer_info,
            pool_info,
            pool_tokens_info,
            mint_info,
            system_program,
            token_program,
            associated_token_program,
        )?;
    } else {
        pool_tokens_info.to_associated_token_account(pool_info.key, mint_info.key)?;
    }

    Ok(())
}
