use ore_pool_api::prelude::*;
use steel::*;

/// Deposit tokens into a pool's pending stake account.
pub fn process_stake(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = ore_pool_api::instruction::Stake::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    // Load accounts.
    let [signer_info, mint_info, member_info, pool_info, pool_tokens_info, sender_tokens_info, share_info, token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    mint_info.to_mint()?;
    member_info
        .to_account::<Member>(&ore_pool_api::ID)?
        .check(|m| m.authority == *signer_info.key)?
        .check(|m| m.pool == *pool_info.key)?;
    pool_info.to_account::<Pool>(&ore_pool_api::ID)?;
    pool_tokens_info
        .is_writable()?
        .to_associated_token_account(pool_info.key, mint_info.key)?;
    sender_tokens_info
        .is_writable()?
        .to_token_account()?
        .check(|t| t.owner == *signer_info.key)?
        .check(|t| t.mint == *mint_info.key)?;
    let share = share_info
        .to_account_mut::<Share>(&ore_pool_api::ID)?
        .check_mut(|s| s.authority == *signer_info.key)?
        .check_mut(|s| s.mint == *mint_info.key)?;
    token_program.is_program(&spl_token::ID)?;

    // Update the share balance.
    share.balance = share.balance.checked_add(amount).unwrap();

    // Transfer tokens into pool's pending stake account.
    transfer(
        signer_info,
        sender_tokens_info,
        pool_tokens_info,
        token_program,
        amount,
    )?;

    Ok(())
}
