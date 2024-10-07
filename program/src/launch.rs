use ore_api::prelude::*;
use ore_pool_api::prelude::*;
use steel::*;

/// Launch creates a new pool.
pub fn process_launch(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = Launch::try_from_bytes(data)?;

    // Load accounts.
    let [signer_info, miner_info, pool_info, proof_info, ore_program, token_program, associated_token_program, system_program, slot_hashes_sysvar] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    pool_info.is_empty()?.is_writable()?.has_seeds(
        &[POOL, signer_info.key.as_ref()],
        args.pool_bump,
        &ore_pool_api::ID,
    )?;
    proof_info.is_empty()?.is_writable()?.has_seeds(
        &[PROOF, pool_info.key.as_ref()],
        args.proof_bump,
        &ore_api::ID,
    )?;
    ore_program.is_program(&ore_api::ID)?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;
    system_program.is_program(&system_program::ID)?;
    slot_hashes_sysvar.is_sysvar(&sysvar::slot_hashes::ID)?;

    // Open proof account.
    solana_program::program::invoke_signed(
        &ore_api::sdk::open(*pool_info.key, *miner_info.key, *signer_info.key),
        &[
            pool_info.clone(),
            miner_info.clone(),
            signer_info.clone(),
            proof_info.clone(),
            system_program.clone(),
            slot_hashes_sysvar.clone(),
        ],
        &[&[POOL, signer_info.key.as_ref(), &[args.pool_bump]]],
    )?;

    // Parse proof.
    let proof = proof_info.to_account::<Proof>(&ore_api::ID)?;

    // Initialize pool account.
    create_account::<Pool>(
        pool_info,
        &ore_pool_api::id(),
        &[POOL, signer_info.key.as_ref(), &[args.pool_bump]],
        system_program,
        signer_info,
    )?;
    let pool = pool_info.to_account_mut::<Pool>(&ore_pool_api::ID)?;
    pool.authority = *signer_info.key;
    pool.bump = args.pool_bump as u64;
    pool.url = args.url;
    pool.attestation = [0; 32];
    pool.last_total_members = 0;
    pool.last_hash_at = proof.last_hash_at;

    Ok(())
}
