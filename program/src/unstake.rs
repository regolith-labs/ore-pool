use ore_boost_legacy_api::state::Boost;
use ore_pool_api::prelude::*;
use solana_program::log::sol_log_data;
use steel::*;

/// Unstake tokens from the pool's stake account.
pub fn process_unstake(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = ore_pool_api::instruction::Unstake::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    // Load accounts.
    let [signer_info, boost_info, boost_tokens_info, mint_info, member_info, pool_info, pool_tokens_info, recipient_tokens_info, share_info, stake_info, token_program, ore_boost_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    boost_info
        .is_writable()?
        .as_account::<Boost>(&ore_boost_legacy_api::ID)?
        .assert(|b| b.mint == *mint_info.key)?;
    boost_tokens_info
        .is_writable()?
        .as_associated_token_account(boost_info.key, mint_info.key)?;
    mint_info.as_mint()?;
    member_info
        .as_account::<Member>(&ore_pool_api::ID)?
        .assert(|m| m.authority == *signer_info.key)?
        .assert(|m| m.pool == *pool_info.key)?;
    let pool = pool_info.as_account::<Pool>(&ore_pool_api::ID)?;
    let pool_tokens = pool_tokens_info
        .is_writable()?
        .as_associated_token_account(pool_info.key, mint_info.key)?;
    recipient_tokens_info
        .is_writable()?
        .as_token_account()?
        .assert(|t| t.mint == *mint_info.key)?;
    stake_info
        .is_writable()?
        .as_account::<ore_boost_legacy_api::state::Stake>(&ore_boost_legacy_api::ID)?
        .assert(|s| s.authority == *pool_info.key)?
        .assert(|s| s.boost == *boost_info.key)?;
    let share = share_info
        .as_account_mut::<Share>(&ore_pool_api::ID)?
        .assert_mut(|s| s.authority == *signer_info.key)?
        .assert_mut(|s| s.pool == *pool_info.key)?
        .assert_mut(|s| s.mint == *mint_info.key)?;
    token_program.is_program(&spl_token::ID)?;
    ore_boost_program.is_program(&ore_boost_legacy_api::ID)?;

    // Update the share balance.
    share.balance = share.balance.checked_sub(amount).unwrap();

    // Check how many pending tokens can be distributed back to staker.
    let pending_amount = pool_tokens.amount.min(amount);
    let withdraw_amount = amount.checked_sub(pending_amount).unwrap();

    // Withdraw remaining amount from staked balance.
    if withdraw_amount.gt(&0) {
        solana_program::program::invoke_signed(
            &ore_boost_legacy_api::sdk::withdraw(*pool_info.key, *mint_info.key, withdraw_amount),
            &[
                pool_info.clone(),
                pool_tokens_info.clone(),
                boost_info.clone(),
                boost_tokens_info.clone(),
                mint_info.clone(),
                stake_info.clone(),
                token_program.clone(),
            ],
            &[&[POOL, pool.authority.as_ref(), &[pool.bump as u8]]],
        )?;
    }

    // Transfer tokens into pool's pending stake account.
    transfer_signed(
        pool_info,
        pool_tokens_info,
        recipient_tokens_info,
        token_program,
        amount,
        &[POOL, pool.authority.as_ref()],
    )?;

    // Log the balance for parsing.
    let event = UnstakeEvent {
        authority: *signer_info.key,
        share: *share_info.key,
        mint: *mint_info.key,
        balance: share.balance,
    };
    sol_log_data(&[event.to_bytes()]);

    Ok(())
}
