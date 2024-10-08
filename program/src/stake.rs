use ore_api::instruction::Stake;
use ore_pool_api::{loaders::*, state::Share};
use steel::*;

/// Deposit tokens into a pool's pending stake account.
pub fn process_stake(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = Stake::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    // Load accounts.
    let [signer, mint_info, member_info, pool_info, pool_tokens_info, sender_tokens_info, share_info, token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer.is_signer()?;
    mint_info.to_mint()?;
    load_member(member_info, signer.key, pool_info.key, false)?;
    load_any_pool(pool_info, false)?;
    pool_tokens_info
        .is_writable()?
        .to_associated_token_account(pool_info.key, mint_info.key)?;
    sender_tokens_info
        .is_writable()?
        .to_associated_token_account(signer.key, mint_info.key)?;
    load_share(share_info, signer.key, pool_info.key, mint_info.key, true)?;
    token_program.is_program(&spl_token::ID)?;

    // Update the share balance.
    let mut share_data = share_info.data.borrow_mut();
    let share = Share::try_from_bytes_mut(&mut share_data)?;
    share.balance = share.balance.checked_add(amount).unwrap();

    // Transfer tokens into pool's pending stake account.
    transfer(
        signer,
        sender_tokens_info,
        pool_tokens_info,
        token_program,
        amount,
    )?;

    Ok(())
}
