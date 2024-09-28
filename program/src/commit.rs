use ore_boost_api::loaders::load_boost;
use ore_pool_api::{consts::POOL, loaders::*, state::Pool};
use ore_utils::*;
use solana_program::{
    self, account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack,
};

/// Commit pending stake from the pool program into the boost program.
pub fn process_commit(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer, boost_info, boost_tokens_info, mint_info, pool_info, pool_tokens_info, stake_info, spl_token_program, ore_boost_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_signer(signer)?;
    load_boost(boost_info, mint_info.key, true)?;
    load_associated_token_account(boost_tokens_info, boost_info.key, mint_info.key, true)?;
    load_any_mint(mint_info, false)?;
    load_pool(pool_info, signer.key, true)?;
    load_associated_token_account(pool_tokens_info, pool_info.key, mint_info.key, true)?;
    load_program(spl_token_program, spl_token::id())?;
    load_program(ore_boost_program, ore_boost_api::id())?;

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
