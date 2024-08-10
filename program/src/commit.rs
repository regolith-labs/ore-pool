use ore_api::*;
use ore_pool_api::{consts::*, instruction::*, loaders::*};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    {self},
};

/// Commit ...
pub fn process_commit<'a, 'info>(accounts: &'a [AccountInfo<'info>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = CommitArgs::try_from_bytes(data)?;

    // Load accounts.
    let [signer, batch_info, member_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_operator(signer)?;
    load_any_batch(batch_info, true)?;
    load_any_member(member_info, false)?;

    // If batch is not certified, then exit
    let batch_data = batch_info.data.borrow();
    let batch = Batch::try_from_bytes(&batch_data)?;
    if batch.attestation.ne(&batch.certification) {
        return Err(PoolError::Dummy.into());
    }

    // TODO Read balance update amount from file
    // TODO Verify pubkey matches member key
    let amount = 0;

    // Update member balance
    let member_data = member_info.data.borrow_mut();
    let member = Member::try_from_bytes_mut(&mut member_data)?;
    member.balance = member.balance.checked_add(amount).unwrap();

    // TODO Free memory from the file
    // TODO Return rent to pool operator

    Ok(())
}
