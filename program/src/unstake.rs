use ore_boost_api::state::{Boost, Stake};
use ore_pool_api::{
    consts::POOL,
    event::UnstakeEvent,
    instruction::*,
    loaders::*,
    state::{Pool, Share},
};
use solana_program::{log::sol_log_data, program::invoke_signed, program_pack::Pack};
use steel::*;

/// Unstake tokens from the pool's stake account.
pub fn process_unstake(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = Unstake::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    // Load accounts.
    let [signer, boost_info, boost_tokens_info, mint_info, member_info, pool_info, pool_tokens_info, recipient_tokens_info, share_info, stake_info, token_program, ore_boost_program] =
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
    load_member(member_info, signer.key, pool_info.key, false)?;
    load_any_pool(pool_info, true)?;
    pool_tokens_info
        .is_writable()?
        .to_associated_token_account(pool_info.key, mint_info.key)?;
    recipient_tokens_info.is_writable()?.to_token_account()?;
    stake_info
        .is_writable()?
        .to_account::<Stake>(&ore_boost_api::ID)?
        .check(|s| s.authority == *pool_info.key)?
        .check(|s| s.boost == *boost_info.key)?;
    load_share(share_info, signer.key, pool_info.key, mint_info.key, true)?;
    token_program.is_program(&spl_token::ID)?;
    ore_boost_program.is_program(&ore_boost_api::ID)?;

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

    // Log the balance for parsing.
    let event = UnstakeEvent {
        authority: *signer.key,
        share: *share_info.key,
        mint: *mint_info.key,
        balance: share.balance,
    };
    let event = event.to_bytes();
    sol_log_data(&[event]);

    Ok(())
}
