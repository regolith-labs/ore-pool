use ore_pool_api::{instruction::*, loaders::*, state::Member};
use ore_utils::{loaders::load_signer, AccountDeserialize};
use solana_program::{
    self, account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
};

/// Attribute updates a member's claimable balance.
///
/// The arguments to this function expect the member's lifetime earnings. This way,
/// the balance can be updated idempotently and duplicate transactions will not result in
/// duplicate earnings.
pub fn process_attribute(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = AttributeArgs::try_from_bytes(data)?;
    let total_balance = u64::from_le_bytes(args.total_balance);

    // Load accounts.
    let [signer, pool_info, member_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_signer(signer)?;
    load_pool(pool_info, signer.key, false)?;
    load_pool_member(member_info, pool_info.key, true)?;

    // Update balance idempotently
    let mut member_data = member_info.data.borrow_mut();
    let member = Member::try_from_bytes_mut(&mut member_data)?;
    let balance_change = total_balance.saturating_sub(member.total_balance);
    member.balance = member.balance.checked_add(balance_change).unwrap();
    member.total_balance = total_balance;

    Ok(())
}
