use drillx::Solution;
use ore_api::loaders::*;
use ore_pool_api::{consts::*, instruction::*, loaders::*, state::Pool};
use ore_utils::{loaders::*, AccountDeserialize};
use solana_program::{
    self, account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    system_program, sysvar,
};

/// Submit sends the pool's best hash to the ORE mining contract.
pub fn process_submit<'a, 'info>(accounts: &'a [AccountInfo<'info>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = SubmitArgs::try_from_bytes(data)?;

    // Load accounts.
    let [signer, bus_info, config_info, pool_info, proof_info, ore_program, system_program, instructions_sysvar, slot_hashes_sysvar] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_signer(signer)?;
    load_any_bus(bus_info, true)?;
    load_config(config_info, false)?;
    load_pool(pool_info, signer.key, true)?;
    load_proof(proof_info, pool_info.key, true)?;
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
        &[&[POOL, signer.key.as_ref(), &[POOL_BUMP]]],
    )?;

    // Update pool submissions count
    let mut pool_data = pool_info.data.borrow_mut();
    let pool = Pool::try_from_bytes_mut(&mut pool_data)?;
    pool.attestation = args.attestation;
    pool.total_submissions = pool.total_submissions.checked_add(1).unwrap();

    Ok(())
}
