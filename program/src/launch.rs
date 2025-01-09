use ore_api::prelude::*;
use ore_boost_api::consts::RESERVATION;
use ore_pool_api::prelude::*;
use steel::*;

/// Launch creates a new pool.
pub fn process_launch(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = Launch::try_from_bytes(data)?;

    // Load accounts.
    let [signer_info, miner_info, pool_info, proof_info, reservation_info, ore_program, ore_boost_program, token_program, associated_token_program, system_program, slot_hashes_sysvar] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    pool_info
        .is_writable()?
        .has_seeds(&[POOL, signer_info.key.as_ref()], &ore_pool_api::ID)?;
    proof_info
        .is_writable()?
        .has_seeds(&[PROOF, pool_info.key.as_ref()], &ore_api::ID)?;
    reservation_info
        .is_writable()?
        .has_seeds(&[RESERVATION, proof_info.key.as_ref()], &ore_boost_api::ID)?;
    ore_program.is_program(&ore_api::ID)?;
    ore_boost_program.is_program(&ore_boost_api::ID)?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;
    system_program.is_program(&system_program::ID)?;
    slot_hashes_sysvar.is_sysvar(&sysvar::slot_hashes::ID)?;

    // Open proof account.
    if proof_info.is_empty().is_ok() {
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
    }

    // Initialize reservation account.
    if reservation_info.is_empty().is_ok() {
        solana_program::program::invoke_signed(
            &ore_boost_api::sdk::register(*signer_info.key, *signer_info.key, *proof_info.key),
            &[
                pool_info.clone(),
                signer_info.clone(),
                proof_info.clone(),
                reservation_info.clone(),
                system_program.clone(),
            ],
            &[&[POOL, signer_info.key.as_ref(), &[args.pool_bump]]],
        )?;
    }

    // Initialize pool account.
    let proof = proof_info.as_account::<Proof>(&ore_api::ID)?;
    if pool_info.is_empty().is_ok() {
        create_account::<Pool>(
            pool_info,
            system_program,
            signer_info,
            &ore_pool_api::id(),
            &[POOL, signer_info.key.as_ref()],
        )?;
        let pool = pool_info.as_account_mut::<Pool>(&ore_pool_api::ID)?;
        pool.authority = *signer_info.key;
        pool.bump = args.pool_bump as u64;
        pool.url = args.url;
        pool.attestation = [0; 32];
        pool.last_total_members = 0;
        pool.last_hash_at = proof.last_hash_at;
    }

    Ok(())
}

