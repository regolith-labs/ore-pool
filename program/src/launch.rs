use std::mem::size_of;

use ore_api::consts::*;
use ore_pool_api::{consts::*, instruction::LaunchArgs, state::Pool};
use ore_utils::{create_pda, loaders::*, AccountDeserialize, Discriminator};
use solana_program::{
    self, account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    system_program, sysvar,
};

/// Launch creates a new pool.
pub fn process_launch(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = LaunchArgs::try_from_bytes(data)?;

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
    let mut pool_data = pool_info.try_borrow_mut_data()?;
    pool_data[0] = Pool::discriminator();
    let pool = Pool::try_from_bytes_mut(&mut pool_data)?;
    pool.authority = *signer.key;
    pool.url = args.url;
    pool.attestation = [0; 32];
    // must be non-zero value
    // to divide nonce space for at least one member
    pool.last_total_members = 1;

    // Open proof account.
    drop(pool_data);
    solana_program::program::invoke_signed(
        &ore_api::instruction::open(*pool_info.key, *miner_info.key, *signer.key),
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

    Ok(())
}
