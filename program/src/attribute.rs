use ore_pool_api::prelude::*;
use steel::*;

/// Attribute updates a member's claimable balance.
///
/// The arguments to this function expect the member's lifetime earnings. This way,
/// the balance can be updated idempotently and duplicate transactions will not result in
/// duplicate earnings.
pub fn process_attribute(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = Attribute::try_from_bytes(data)?;
    let total_balance = u64::from_le_bytes(args.total_balance);

    // Load accounts.
    let [signer_info, pool_authority_info, pool_info, member_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    pool_authority_info.is_signer()?;
    pool_info
        .as_account::<Pool>(&ore_pool_api::ID)?
        .assert(|p| p.authority == *pool_authority_info.key)?;
    let member = member_info
        .as_account_mut::<Member>(&ore_pool_api::ID)?
        .assert_mut(|m| m.pool == *pool_info.key)?;

    // Update balance idempotently
    let balance_change = total_balance.saturating_sub(member.total_balance);
    member.balance = member.balance.checked_add(balance_change).unwrap();
    member.total_balance = total_balance;

    Ok(())
}
