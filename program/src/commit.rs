use ore_boost_api::state::Boost;
use ore_pool_api::prelude::*;
use steel::*;

/// Commit pending stake from the pool program into the boost program.
pub fn process_commit(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer_info, boost_info, boost_tokens_info, mint_info, pool_info, pool_tokens_info, stake_info, spl_token_program, ore_boost_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    boost_info
        .is_writable()?
        .to_account::<Boost>(&ore_boost_api::ID)?
        .check(|b| b.mint == *mint_info.key)?;
    boost_tokens_info
        .is_writable()?
        .to_associated_token_account(boost_info.key, mint_info.key)?;
    mint_info.to_mint()?;
    let pool = pool_info
        .to_account_mut::<Pool>(&ore_pool_api::ID)?
        .check_mut(|p| p.authority == *signer_info.key)?;
    let pool_tokens = pool_tokens_info
        .is_writable()?
        .to_associated_token_account(pool_info.key, mint_info.key)?;
    spl_token_program.is_program(&spl_token::ID)?;
    ore_boost_program.is_program(&ore_boost_api::ID)?;

    // Deposit stake into ORE boost program.
    solana_program::program::invoke_signed(
        &ore_boost_api::sdk::deposit(*pool_info.key, *mint_info.key, pool_tokens.amount),
        &[
            pool_info.clone(),
            boost_info.clone(),
            boost_tokens_info.clone(),
            mint_info.clone(),
            pool_tokens_info.clone(),
            stake_info.clone(),
            spl_token_program.clone(),
        ],
        &[&[POOL, signer_info.key.as_ref(), &[pool.bump as u8]]],
    )?;

    Ok(())
}
