use ore_api::state::Proof;
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
    let [signer_info, pool_info, proof_info, member_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    let pool = pool_info
        .as_account_mut::<Pool>(&ore_pool_api::ID)?
        .assert_mut(|p| p.authority == *signer_info.key)?;
    let proof = proof_info
        .is_writable()?
        .as_account::<Proof>(&ore_api::ID)?
        .assert(|p| p.authority == *pool_info.key)?;
    let member = member_info
        .as_account_mut::<Member>(&ore_pool_api::ID)?
        .assert_mut(|m| m.pool == *pool_info.key)?;

    // Update balance idempotently
    let balance_change = total_balance.checked_sub(member.total_balance).unwrap();
    member.balance = member.balance.checked_add(balance_change).unwrap();
    member.total_balance = total_balance;

    // Update claimable balance
    pool.total_rewards = pool.total_rewards.checked_add(balance_change).unwrap();

    // Validate there are claimable rewards in the proof account for this attribution.
    if pool.total_rewards > proof.balance {
        return Err(PoolError::AttributionTooLarge.into());
    }

    Ok(())
}
