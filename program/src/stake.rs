use base64::{prelude::BASE64_STANDARD, Engine};
use ore_api::instruction::Stake;
use ore_pool_api::{event::StakeEvent, loaders::*, state::Share};
use ore_utils::*;
use solana_program::{
    self, account_info::AccountInfo, entrypoint::ProgramResult, log::sol_log,
    program_error::ProgramError,
};

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
    load_signer(signer)?;
    load_any_mint(mint_info, false)?;
    load_member(member_info, signer.key, pool_info.key, false)?;
    load_any_pool(pool_info, false)?;
    load_associated_token_account(pool_tokens_info, pool_info.key, mint_info.key, true)?;
    load_associated_token_account(sender_tokens_info, signer.key, mint_info.key, true)?;
    load_share(share_info, signer.key, pool_info.key, mint_info.key, true)?;
    load_program(token_program, spl_token::id())?;

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

    // Log the balance for parsing.
    let event = StakeEvent {
        authority: *signer.key,
        share: *share_info.key,
        mint: *mint_info.key,
        balance: share.balance,
    };
    let event = event.to_bytes();
    let event = BASE64_STANDARD.encode(event);
    sol_log(event.as_str());

    Ok(())
}
