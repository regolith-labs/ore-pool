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
        .has_owner(&LEGACY_BOOST_PROGRAM_ID)?;
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
        .has_owner(&LEGACY_BOOST_PROGRAM_ID)?;
    let share = share_info
        .as_account_mut::<Share>(&ore_pool_api::ID)?
        .assert_mut(|s| s.authority == *signer_info.key)?
        .assert_mut(|s| s.pool == *pool_info.key)?
        .assert_mut(|s| s.mint == *mint_info.key)?;
    token_program.is_program(&spl_token::ID)?;
    ore_boost_program.is_program(&LEGACY_BOOST_PROGRAM_ID)?;

    // Update the share balance.
    share.balance = share.balance.checked_sub(amount).unwrap();

    // Check how many pending tokens can be distributed back to staker.
    let pending_amount = pool_tokens.amount.min(amount);
    let withdraw_amount = amount.checked_sub(pending_amount).unwrap();

    // Withdraw remaining amount from staked balance.
    if withdraw_amount.gt(&0) {
        invoke_signed(
            // &ore_boost_legacy_api::sdk::withdraw(*pool_info.key, *mint_info.key, withdraw_amount),
            &Instruction {
                program_id: LEGACY_BOOST_PROGRAM_ID,
                accounts: vec![
                    AccountMeta::new(*pool_info.key, true),
                    AccountMeta::new(*pool_tokens_info.key, false),
                    AccountMeta::new(*boost_info.key, false),
                    AccountMeta::new(*boost_tokens_info.key, false),
                    AccountMeta::new_readonly(*mint_info.key, false),
                    AccountMeta::new(*stake_info.key, false),
                    AccountMeta::new_readonly(*token_program.key, false),
                ],
                data: [
                    [3 as u8].to_vec(),
                    bytemuck::bytes_of(&withdraw_amount).to_vec(),
                ]
                .concat(),
            },
            &[
                pool_info.clone(),
                pool_tokens_info.clone(),
                boost_info.clone(),
                boost_tokens_info.clone(),
                mint_info.clone(),
                stake_info.clone(),
                token_program.clone(),
            ],
            &ore_pool_api::ID,
            &[POOL, pool.authority.as_ref()],
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
