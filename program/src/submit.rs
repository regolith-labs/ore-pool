use drillx::Solution;
use ore_api::prelude::*;
use ore_pool_api::prelude::*;
use steel::*;

/// Submit sends the pool's best hash to the ORE mining contract.
pub fn process_submit(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = Submit::try_from_bytes(data)?;

    // Load accounts.
    let (required_accounts, boost_accounts) = accounts.split_at(9);
    let [signer_info, bus_info, config_info, pool_info, proof_info, ore_program, system_program, instructions_sysvar, slot_hashes_sysvar] =
        required_accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    let pool = pool_info
        .as_account_mut::<Pool>(&ore_pool_api::ID)?
        .assert_mut(|p| p.authority == *signer_info.key)?;
    let proof = proof_info
        .is_writable()?
        .as_account::<Proof>(&ore_api::ID)?
        .assert(|p| p.authority == *pool_info.key)?;
    ore_program.is_program(&ore_api::ID)?;
    system_program.is_program(&system_program::ID)?;
    instructions_sysvar.is_sysvar(&sysvar::instructions::ID)?;
    slot_hashes_sysvar.is_sysvar(&sysvar::slot_hashes::ID)?;

    // Parse boost accounts
    let [boost_config, boost_proof] = boost_accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Build solution for submitting to the ORE program
    let solution = Solution::new(args.digest, args.nonce);

    // Invoke mine CPI
    solana_program::program::invoke(
        &ore_api::sdk::mine(
            *signer_info.key,
            *pool_info.key,
            *bus_info.key,
            solution,
            *boost_config.key,
        ),
        &[
            signer_info.clone(),
            bus_info.clone(),
            config_info.clone(),
            proof_info.clone(),
            instructions_sysvar.clone(),
            slot_hashes_sysvar.clone(),
            boost_config.clone(),
            boost_proof.clone(),
        ],
    )?;

    // Update pool state.
    pool.attestation = args.attestation;
    pool.last_hash_at = proof.last_hash_at;
    pool.last_total_members = pool.total_members;
    pool.total_submissions += 1;

    Ok(())
}
