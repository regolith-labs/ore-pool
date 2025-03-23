use ore_api::prelude::*;
use ore_pool_api::prelude::*;
use steel::*;

/// Claim allows a member to claim their ORE rewards from the pool.
pub fn process_claim(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = ore_pool_api::instruction::Claim::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    // Load accounts.
    let [signer_info, beneficiary_info, member_info, pool_info, pool_tokens_info, proof_info, treasury_info, treasury_tokens_info, ore_program, token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    beneficiary_info
        .is_writable()?
        .as_token_account()?
        .assert(|t| t.mint() == MINT_ADDRESS)?;
    let member: &mut Member = member_info
        .as_account_mut::<Member>(&ore_pool_api::ID)?
        .assert_mut(|m| m.authority == *signer_info.key)?
        .assert_mut(|m| m.pool == *pool_info.key)?;
    let pool = pool_info.as_account_mut::<Pool>(&ore_pool_api::ID)?;
    proof_info
        .as_account::<Proof>(&ore_api::ID)?
        .assert(|p| p.authority == *pool_info.key)?;
    treasury_info.has_address(&ore_api::consts::TREASURY_ADDRESS)?;
    treasury_tokens_info.has_address(&ore_api::consts::TREASURY_TOKENS_ADDRESS)?;
    ore_program.is_program(&ore_api::ID)?;
    token_program.is_program(&spl_token::ID)?;

    // Update member balance
    member.balance -= amount;

    // Update pool balance
    pool.total_rewards -= amount;

    // Amount claimed
    let mut amount_claimed = 0;

    // Claim first from pool tokens
    if !pool_tokens_info.data_is_empty() {
        // Verify pool tokens account
        let pool_tokens = pool_tokens_info
            .is_writable()?
            .as_associated_token_account(pool_info.key, &MINT_ADDRESS)?;

        // Calculate how much we can claim from pool tokens
        amount_claimed = pool_tokens.amount().min(amount);

        // Transfer available tokens from pool to beneficiary
        if amount_claimed > 0 {
            transfer_signed(
                pool_info,
                pool_tokens_info,
                beneficiary_info,
                token_program,
                amount_claimed,
                &[POOL, pool.authority.as_ref()],
            )?;
        }
    }

    // If we still have tokens to claim after claiming from pool tokens, claim from ore program
    let remaining_amount = amount - amount_claimed;
    if remaining_amount > 0 {
        invoke_signed(
            &ore_api::sdk::claim(*pool_info.key, *beneficiary_info.key, remaining_amount),
            &[
                pool_info.clone(),
                beneficiary_info.clone(),
                proof_info.clone(),
                treasury_info.clone(),
                treasury_tokens_info.clone(),
                token_program.clone(),
            ],
            &ore_pool_api::ID,
            &[POOL, pool.authority.as_ref()],
        )?;
    }

    Ok(())
}
