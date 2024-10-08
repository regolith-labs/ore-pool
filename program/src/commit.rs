use ore_boost_api::state::Boost;
use ore_pool_api::{consts::POOL, loaders::*, state::Pool};
use solana_program::program_pack::Pack;
use steel::*;

/// Commit pending stake from the pool program into the boost program.
pub fn process_commit(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer, boost_info, boost_tokens_info, mint_info, pool_info, pool_tokens_info, stake_info, spl_token_program, ore_boost_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer.is_signer()?;
    boost_info
        .is_writable()?
        .to_account::<Boost>(&ore_boost_api::ID)?
        .check(|b| b.mint == *mint_info.key)?;
    boost_tokens_info
        .is_writable()?
        .to_associated_token_account(boost_info.key, mint_info.key)?;
    mint_info.to_mint()?;
    load_pool(pool_info, signer.key, true)?;
    pool_tokens_info
        .is_writable()?
        .to_associated_token_account(pool_info.key, mint_info.key)?;
    spl_token_program.is_program(&spl_token::ID)?;
    ore_boost_program.is_program(&ore_boost_api::ID)?;

    // Load the pool bump
    let pool_data = pool_info.data.borrow();
    let pool = Pool::try_from_bytes(&pool_data)?;
    let pool_bump = pool.bump as u8;

    // Load the token amount.
    let pool_tokens_data = pool_tokens_info.data.borrow();
    let pool_tokens = spl_token::state::Account::unpack(&pool_tokens_data)?;
    let amount = pool_tokens.amount;

    // Deposit stake into ORE boost program.
    drop(pool_data);
    drop(pool_tokens_data);
    solana_program::program::invoke_signed(
        &ore_boost_api::sdk::deposit(*pool_info.key, *mint_info.key, amount),
        &[
            pool_info.clone(),
            boost_info.clone(),
            boost_tokens_info.clone(),
            mint_info.clone(),
            pool_tokens_info.clone(),
            stake_info.clone(),
            spl_token_program.clone(),
        ],
        &[&[POOL, signer.key.as_ref(), &[pool_bump]]],
    )?;

    Ok(())
}
