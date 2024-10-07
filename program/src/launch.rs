use std::mem::size_of;

use ore_api::{consts::*, state::Proof};
use ore_pool_api::{consts::*, instruction::Launch, state::Pool};
use ore_utils::{
    create_pda, load_any, load_program, load_signer, load_sysvar, load_uninitialized_pda,
    AccountDeserialize, Discriminator,
};
use solana_program::{
    self, account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    system_program, sysvar,
};

/// Launch creates a new pool.
pub fn process_launch(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = Launch::try_from_bytes(data)?;

    // Load accounts.
    let [signer, miner_info, pool_info, proof_info, ore_program, token_program, associated_token_program, system_program, slot_hashes_sysvar] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_signer(signer)?;
    load_any(miner_info, false)?;
    load_uninitialized_pda(
        pool_info,
        &[POOL, signer.key.as_ref()],
        args.pool_bump,
        &ore_pool_api::id(),
    )?;
    load_uninitialized_pda(
        proof_info,
        &[PROOF, pool_info.key.as_ref()],
        args.proof_bump,
        &ore_api::id(),
    )?;
    load_program(ore_program, ore_api::id())?;
    load_program(token_program, spl_token::id())?;
    load_program(associated_token_program, spl_associated_token_account::id())?;
    load_program(system_program, system_program::id())?;
    load_sysvar(slot_hashes_sysvar, sysvar::slot_hashes::id())?;

    // Initialize pool account.
    create_pda(
        pool_info,
        &ore_pool_api::id(),
        8 + size_of::<Pool>(),
        &[POOL, signer.key.as_ref(), &[args.pool_bump]],
        system_program,
        signer,
    )?;

    // Open proof account.
    solana_program::program::invoke_signed(
        &ore_api::sdk::open(*pool_info.key, *miner_info.key, *signer.key),
        &[
            pool_info.clone(),
            miner_info.clone(),
            signer.clone(),
            proof_info.clone(),
            system_program.clone(),
            slot_hashes_sysvar.clone(),
        ],
        &[&[POOL, signer.key.as_ref(), &[args.pool_bump]]],
    )?;

    // Parse proof account for last-hash-at.
    let mut proof_data = proof_info.try_borrow_mut_data()?;
    let proof = Proof::try_from_bytes_mut(&mut proof_data)?;
    let last_hash_at = proof.last_hash_at;

    // Initialize pool account data.
    let mut pool_data = pool_info.try_borrow_mut_data()?;
    pool_data[0] = Pool::discriminator();
    let pool = Pool::try_from_bytes_mut(&mut pool_data)?;
    pool.authority = *signer.key;
    pool.bump = args.pool_bump as u64;
    pool.url = args.url;
    pool.attestation = [0; 32];
    pool.last_total_members = 0;
    pool.last_hash_at = last_hash_at;

    Ok(())
}
