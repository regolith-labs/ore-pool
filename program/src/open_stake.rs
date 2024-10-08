use ore_boost_api::state::Boost;
use ore_pool_api::{consts::*, loaders::load_pool, state::Pool};
use solana_program::program::invoke_signed;
use steel::*;

/// Opens a new stake account for the pool in the boost program.
pub fn process_open_stake(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer, boost_info, mint_info, pool_info, pool_tokens_info, stake_info, system_program, token_program, associated_token_program, ore_boost_program] =
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
    load_pool(pool_info, signer.key, true)?;
    pool_tokens_info.is_writable()?;
    stake_info.is_writable()?;
    system_program.is_program(&system_program::ID)?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;
    ore_boost_program.is_program(&ore_boost_api::ID)?;

    // Load pool bump for signing CPIs.
    let pool_data = pool_info.data.borrow();
    let pool = Pool::try_from_bytes(&pool_data)?;
    let pool_bump = pool.bump as u8;
    drop(pool_data);

    // Open the stake account.
    invoke_signed(
        &ore_boost_api::sdk::open(*pool_info.key, *signer.key, *mint_info.key),
        &[
            pool_info.clone(),
            signer.clone(),
            boost_info.clone(),
            mint_info.clone(),
            stake_info.clone(),
            system_program.clone(),
        ],
        &[&[POOL, signer.key.as_ref(), &[pool_bump]]],
    )?;

    // Create token account for pending pool stake, if necessary
    if pool_tokens_info.data.borrow().is_empty() {
        create_associated_token_account(
            signer,
            pool_info,
            pool_tokens_info,
            mint_info,
            system_program,
            token_program,
            associated_token_program,
        )?;
    } else {
        pool_tokens_info
            .is_writable()?
            .to_associated_token_account(pool_info.key, mint_info.key)?;
    }

    Ok(())
}
