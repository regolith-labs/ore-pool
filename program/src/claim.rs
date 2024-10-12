use ore_api::prelude::*;
use ore_pool_api::prelude::*;
use steel::*;

/// Claim allows a member to claim their ORE rewards from the pool.
pub fn process_claim(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = ore_pool_api::instruction::Claim::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    // Load accounts.
    let [signer_info, beneficiary_info, member_info, pool_info, proof_info, treasury_info, treasury_tokens_info, ore_program, token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    beneficiary_info
        .is_writable()?
        .to_token_account()?
        .check(|t| t.mint == MINT_ADDRESS)?;
    let member = member_info
        .to_account_mut::<Member>(&ore_pool_api::ID)?
        .check_mut(|m| m.authority == *signer_info.key)?
        .check_mut(|m| m.pool == *pool_info.key)?;
    let pool = pool_info.to_account::<Pool>(&ore_pool_api::ID)?;
    ore_program.is_program(&ore_api::ID)?;
    token_program.is_program(&spl_token::ID)?;

    // Update member balance
    member.balance = member.balance.checked_sub(amount).unwrap();

    // Claim tokens to the beneficiary
    let pool_authority = pool.authority;
    solana_program::program::invoke_signed(
        &ore_api::sdk::claim(*pool_info.key, *beneficiary_info.key, amount),
        &[
            pool_info.clone(),
            beneficiary_info.clone(),
            proof_info.clone(),
            treasury_info.clone(),
            treasury_tokens_info.clone(),
            token_program.clone(),
        ],
        &[&[POOL, pool_authority.as_ref(), &[args.pool_bump]]],
    )?;

    Ok(())
}
