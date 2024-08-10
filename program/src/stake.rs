use ore_api::*;
use ore_pool_api::{consts::*, instruction::*, loaders::*};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    {self},
};

/// Stake ...
pub fn process_stake<'a, 'info>(accounts: &'a [AccountInfo<'info>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = StakeArgs::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    // Load accounts.
    let [signer, authority_info, member_info, pool_info, proof_info, sender_info, treasury_info, treasury_tokens_info, ore_program, token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    // TODO Account loaders
    // load_operator(signer)?;
    // load_any(miner_info)?;
    // load_uninitialized_pda(pool_info, &[POOL], args.pool_bump, &ore_pool_api::id())?;
    // load_program(system_program, system_program::id())?;

    // Update member balance
    let mut member_data = member_info.try_borrow_mut_data()?;
    let member = Member::try_from_bytes_mut(&mut member_data)?;
    member.balance = member.balance.checked_add(amount).unwrap();

    // Stake tokens to the pool
    solana_program::program::invoke_signed(
        &ore_api::instruction::stake(*pool_info.key, *sender_info.key, amount),
        &[
            pool_info.clone(),
            proof_info.clone(),
            sender_info.clone(),
            treasury_tokens_info.clone(),
            token_program.clone(),
        ],
        &[&[POOL, &[POOL_BUMP]]],
    )?;

    Ok(())
}
