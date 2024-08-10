use ore_api::*;
use ore_pool_api::{consts::*, instruction::*, loaders::*};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    {self},
};

/// Claim ...
pub fn process_claim<'a, 'info>(accounts: &'a [AccountInfo<'info>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = ClaimArgs::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    // Load accounts.
    let [signer, authority_info, beneficiary_info, member_info, pool_info, proof_info, treasury_info, treasury_tokens_info, ore_program, token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    // TODO Account loaders
    // load_operator(signer)?;
    // load_any(miner_info)?;
    // load_uninitialized_pda(pool_info, &[POOL], args.pool_bump, &ore_pool_api::id())?;
    // load_program(system_program, system_program::id())?;

    // Verify claim amount
    let mut member_data = member_info.try_borrow_mut_data()?;
    let member = Member::try_from_bytes_mut(&mut member_data)?;
    member.balance = member.balance.checked_sub(amount).unwrap();

    // Claim tokens to the beneficiary
    solana_program::program::invoke_signed(
        &ore_api::instruction::claim(*pool_info.key, *beneficiary_info.key, amount),
        &[
            pool_info.clone(),
            beneficiary_info.clone(),
            proof_info.clone(),
            treasury_info.clone(),
            treasury_tokens_info.clone(),
            token_program.clone(),
        ],
        &[&[POOL, &[POOL_BUMP]]],
    )?;

    Ok(())
}
