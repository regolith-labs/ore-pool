use ore_api::{consts::*, loaders::OreAccountInfoValidation};
use ore_pool_api::{
    consts::*,
    instruction::*,
    loaders::*,
    state::{Member, Pool},
};
use steel::*;

/// Claim allows a member to claim their ORE from the pool.
pub fn process_claim(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = Claim::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    // Load accounts.
    let [signer, beneficiary_info, member_info, pool_info, proof_info, treasury_info, treasury_tokens_info, ore_program, token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer.is_signer()?;
    beneficiary_info
        .is_writable()?
        .to_token_account()?
        .check(|t| t.mint == MINT_ADDRESS)?;
    load_member(member_info, signer.key, pool_info.key, true)?;
    load_any_pool(pool_info, true)?;
    treasury_info.is_treasury()?;
    treasury_tokens_info.is_writable()?.is_treasury_tokens()?;
    ore_program.is_program(&ore_api::ID)?;
    token_program.is_program(&spl_token::ID)?;

    // Update member balance
    let mut member_data = member_info.try_borrow_mut_data()?;
    let member = Member::try_from_bytes_mut(&mut member_data)?;
    member.balance = member.balance.checked_sub(amount).unwrap();

    // Claim tokens to the beneficiary
    let pool_data = pool_info.try_borrow_data()?;
    let pool = Pool::try_from_bytes(&pool_data)?;
    let pool_authority = pool.authority;
    drop(pool_data);
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
