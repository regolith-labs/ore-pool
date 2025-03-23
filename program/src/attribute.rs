use ore_api::prelude::*;
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
    let [signer_info, pool_info, pool_tokens_info, proof_info, member_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    let pool = pool_info
        .as_account_mut::<Pool>(&ore_pool_api::ID)?
        .assert_mut(|p| p.authority == *signer_info.key)?;
    let proof = proof_info
        .as_account::<Proof>(&ore_api::ID)?
        .assert(|p| p.authority == *pool_info.key)?;
    let member = member_info
        .as_account_mut::<Member>(&ore_pool_api::ID)?
        .assert_mut(|m| m.pool == *pool_info.key)?
        .assert_mut(|m| total_balance >= m.total_balance)?;

    // Update balance idempotently
    let balance_change = total_balance - member.total_balance;
    member.balance += balance_change;
    member.total_balance = total_balance;

    // Update claimable balance
    pool.total_rewards += balance_change;

    // Calculate the total reserves of the pool.
    let reserves = if pool_tokens_info.data_is_empty() {
        proof.balance
    } else {
        let pool_tokens =
            pool_tokens_info.as_associated_token_account(pool_info.key, &MINT_ADDRESS)?;
        proof.balance + pool_tokens.amount()
    };

    // Validate there are enough reserves to cover the total rewards owed to miners.
    // This protects pool members from the scenario of a malicious pool operator or compromised key
    // stealing previously attributed member rewards.
    if pool.total_rewards > reserves {
        return Err(PoolError::AttributionTooLarge.into());
    }

    Ok(())
}
