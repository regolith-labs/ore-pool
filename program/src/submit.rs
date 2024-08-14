use std::mem::size_of;

use drillx::Solution;
use ore_api::{event::MineEvent, loaders::*};
use ore_pool_api::{
    consts::*,
    error::PoolError,
    instruction::*,
    loaders::*,
    state::{Pool, Submission},
};
use ore_utils::{create_pda, AccountDeserialize, Discriminator};
use solana_program::{
    self, account_info::AccountInfo, entrypoint::ProgramResult, program::get_return_data,
    program_error::ProgramError, system_program, sysvar,
};

/// Submit sends the pool's best hash to the ORE mining contract.
///
/// This instruction requires the operator to provide an attestation about the hashpower that participated
/// in the pool. A submission account is created to hold the attestation for later certification. No member
/// is allowed to claim their slice of the ORE in this submission until the attestation is certified.
pub fn process_submit<'a, 'info>(accounts: &'a [AccountInfo<'info>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = SubmitArgs::try_from_bytes(data)?;

    // Load accounts.
    let [signer, bus_info, config_info, pool_info, proof_info, submission_info, ore_program, system_program, instructions_sysvar, slot_hashes_sysvar] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_operator(signer)?;
    load_any_bus(bus_info, true)?;
    load_config(config_info, false)?;
    load_pool(pool_info, true)?;
    load_proof(proof_info, pool_info.key, true)?;
    load_uninitialized_pda(
        submission_info,
        &[SUBMISSION],
        args.submission_bump,
        &ore_pool_api::id(),
    )?;
    load_program(ore_program, ore_api::id())?;
    load_program(system_program, system_program::id())?;
    load_sysvar(instructions_sysvar, sysvar::instructions::id())?;
    load_sysvar(slot_hashes_sysvar, sysvar::slot_hashes::id())?;

    // Submit solution to the ORE program
    let solution = Solution::new(args.digest, args.nonce);
    solana_program::program::invoke_signed(
        &ore_api::instruction::mine(*pool_info.key, *pool_info.key, *bus_info.key, solution),
        &[
            pool_info.clone(),
            bus_info.clone(),
            config_info.clone(),
            proof_info.clone(),
            instructions_sysvar.clone(),
            slot_hashes_sysvar.clone(),
        ],
        &[&[POOL, &[POOL_BUMP]]],
    )?;

    // Load the return data
    let Some((program_id, data)) = get_return_data() else {
        return Err(PoolError::Dummy.into());
    };
    if program_id.ne(&ore_api::id()) {
        return Err(PoolError::Dummy.into());
    }
    let Ok(event) = bytemuck::try_from_bytes::<MineEvent>(&data) else {
        return Err(PoolError::Dummy.into());
    };
    let amount = event.reward;

    // Update pool submissions count
    let mut pool_data = pool_info.data.borrow_mut();
    let pool = Pool::try_from_bytes_mut(&mut pool_data)?;
    let submission_id = pool.total_submissions;
    pool.total_submissions = pool.total_submissions.checked_add(1).unwrap();

    // Initialize the submission account
    drop(pool_data);
    create_pda(
        submission_info,
        &ore_pool_api::id(),
        8 + size_of::<Submission>(),
        &[
            SUBMISSION,
            submission_id.to_le_bytes().as_slice(),
            &[args.submission_bump],
        ],
        system_program,
        signer,
    )?;
    let mut submission_data = config_info.data.borrow_mut();
    submission_data[0] = Submission::discriminator() as u8;
    let submission = Submission::try_from_bytes_mut(&mut submission_data)?;
    submission.amount = amount;
    submission.attestation = args.attestation;
    submission.id = submission_id;

    // Update the bool submission count
    let mut pool_data = pool_info.data.borrow_mut();
    let pool = Pool::try_from_bytes_mut(&mut pool_data)?;
    pool.total_submissions = pool.total_submissions.checked_add(1).unwrap();

    Ok(())
}
