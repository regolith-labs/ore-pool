use ore_api::*;
use ore_pool_api::{consts::*, instruction::*, loaders::*};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    {self},
};

/// Submit ...
pub fn process_submit<'a, 'info>(accounts: &'a [AccountInfo<'info>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = SubmitArgs::try_from_bytes(data)?;

    // Load accounts.
    let [signer, batch_info, bus_info, config_info, miner_info, pool_info, proof_info, ore_program, system_program, instructions_sysvar, slot_hashes_sysvar] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_operator(signer)?;
    load_signer(miner_info)?;
    load_program(ore_program, ore_api::id())?;
    load_program(system_program, system_program::id())?;
    load_sysvar(instructions_sysvar, sysvar::instructions::id())?;
    load_sysvar(slot_hashes_sysvar, sysvar::slot_hashes::id())?;
    // TODO Account loaders

    // Load pool data
    let pool_data = pool_info.data.borrow();
    let pool = Pool::try_from_bytes(&pool_data)?;
    let batch_id = pool.total_batches;

    // Load proof data
    let proof_data = proof_info.data.borrow();
    let proof = Proof::try_from_bytes(&proof_data)?;
    let challenge = proof.challenge;
    let balance_pre = proof.balance;

    // Submit solution to the ORE program
    let solution = drillx::Solution {
        d: args.digest,
        n: args.nonce,
    };
    drop(pool_data);
    drop(proof_data);
    solana_program::program::invoke_signed(
        &ore_api::instruction::mine(*pool_info.key, *pool_info.key, *bus_info.key, solution)
            & [
                pool_info.clone(),
                bus_info.clone(),
                config_info.clone(),
                proof_info.clone(),
                instructions_sysvar.clone(),
                slot_hashes_sysvar.clone(),
            ],
        &[&[POOL, &[POOL_BUMP]]],
    )?;

    // Reload proof data
    let proof_data = proof_info.data.borrow();
    let proof = Proof::try_from_bytes(&proof_data)?;
    let balance_post = proof.balance;
    let balance_change = balance_post.checked_sub(balance_pre).unwrap();
    let challenge = proof.challenge;

    // Initialize the batch account
    create_pda(
        batch_info,
        &ore_pool_api::id(),
        8 + size_of::<Batch>(),
        &[BATCH, batch_id, &[args.batch_bump]],
        system_program,
        signer,
    )?;
    let mut batch_data = config_info.data.borrow_mut();
    batch_data[0] = Batch::discriminator() as u8;
    let batch = Batch::try_from_bytes_mut(&mut batch_data)?;
    batch.amount = balance_change;
    batch.attestation = args.attestation;
    batch.challenge = challenge;
    batch.id = batch_id;

    // Update the bool batch count
    let mut pool_data = pool_info.data.borrow_mut();
    let pool = Pool::try_from_bytes_mut(&mut pool_data)?;
    pool.total_batches = pool.total_batches.checked_add(1).unwrap();

    Ok(())
}
