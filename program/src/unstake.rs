use ore_boost_api::loaders::{load_boost, load_stake};
use ore_pool_api::{
    consts::POOL,
    instruction::*,
    loaders::*,
    state::{Pool, Share},
};
use ore_utils::{
    load_any_mint, load_associated_token_account, load_program, load_signer, load_token_account,
    transfer_signed, AccountDeserialize,
};
use solana_program::{
    self, account_info::AccountInfo, entrypoint::ProgramResult, program::invoke_signed,
    program_error::ProgramError, program_pack::Pack,
};

/// Unstake tokens from the pool's stake account.
pub fn process_unstake(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = Stake::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    // Load accounts.
    let [signer, boost_info, boost_tokens_info, mint_info, member_info, pool_info, pool_tokens_info, recipient_tokens_info, share_info, stake_info, token_program, ore_boost_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_signer(signer)?;
    load_boost(boost_info, mint_info.key, true)?;
    load_associated_token_account(boost_tokens_info, boost_info.key, mint_info.key, true)?;
    load_any_mint(mint_info, false)?;
    load_member(member_info, signer.key, pool_info.key, false)?;
    load_any_pool(pool_info, false)?;
    load_associated_token_account(pool_tokens_info, pool_info.key, mint_info.key, true)?;
    load_token_account(recipient_tokens_info, None, mint_info.key, true)?;
    load_stake(stake_info, pool_info.key, boost_info.key, true)?;
    load_share(share_info, signer.key, pool_info.key, mint_info.key, true)?;
    load_program(token_program, spl_token::id())?;
    load_program(ore_boost_program, ore_boost_api::id())?;

    // Update the share balance.
    let mut share_data = share_info.data.borrow_mut();
    let share = Share::try_from_bytes_mut(&mut share_data)?;
    share.balance = share.balance.checked_sub(amount).unwrap();

    // Get pool values for signing CPIs.
    let pool_data = pool_info.data.borrow();
    let pool = Pool::try_from_bytes(&pool_data)?;
    let pool_authority = pool.authority;
    let pool_bump = pool.bump as u8;
    drop(pool_data);

    // Check how many pending tokens can be distributed back to staker.
    let pool_tokens_data = pool_tokens_info.data.borrow();
    let pool_tokens = spl_token::state::Account::unpack(&pool_tokens_data)?;
    let pending_amount = pool_tokens.amount.min(amount);
    let withdraw_amount = amount.checked_sub(pending_amount).unwrap();
    drop(pool_tokens_data);

    // Withdraw remaining amount from staked balance.
    if withdraw_amount.gt(&0) {
        invoke_signed(
            &ore_boost_api::sdk::withdraw(*pool_info.key, *mint_info.key, withdraw_amount),
            &[
                pool_info.clone(),
                pool_tokens_info.clone(),
                boost_info.clone(),
                boost_tokens_info.clone(),
                mint_info.clone(),
                stake_info.clone(),
                token_program.clone(),
            ],
            &[&[POOL, pool_authority.as_ref(), &[pool_bump]]],
        )?;
    }

    // Transfer tokens into pool's pending stake account.
    transfer_signed(
        pool_info,
        pool_tokens_info,
        recipient_tokens_info,
        token_program,
        amount,
        &[&[POOL, pool_authority.as_ref(), &[pool_bump]]],
    )?;

    Ok(())
}
