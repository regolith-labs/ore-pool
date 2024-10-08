use drillx::Solution;
use ore_api::loaders::OreAccountInfoValidation;
use ore_api::state::{Bus, Proof};
use ore_pool_api::{instruction::*, loaders::*, state::Pool};
use steel::*;

/// Submit sends the pool's best hash to the ORE mining contract.
pub fn process_submit(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = Submit::try_from_bytes(data)?;

    // Load accounts.
    let (required_accounts, optional_accounts) = accounts.split_at(9);
    let [signer, bus_info, config_info, pool_info, proof_info, ore_program, system_program, instructions_sysvar, slot_hashes_sysvar] =
        required_accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer.is_signer()?;
    bus_info.is_writable()?.to_account::<Bus>(&ore_api::ID)?;
    config_info.is_config()?;
    load_pool(pool_info, signer.key, true)?;
    proof_info
        .is_writable()?
        .to_account::<Proof>(&ore_api::ID)?
        .check(|p| p.authority == *pool_info.key)?;
    ore_program.is_program(&ore_api::ID)?;
    system_program.is_program(&system_program::ID)?;
    instructions_sysvar.is_sysvar(&sysvar::instructions::ID)?;
    slot_hashes_sysvar.is_sysvar(&sysvar::slot_hashes::ID)?;

    // Update pool submissions count
    let mut pool_data = pool_info.data.borrow_mut();
    let pool = Pool::try_from_bytes_mut(&mut pool_data)?;
    pool.total_submissions = pool.total_submissions.checked_add(1).unwrap();
    // And the attestation of observed hash-power
    pool.attestation = args.attestation;

    // Parse the proof balance before submitting solution
    // as previous balance to compute reward.
    let mut proof_data = proof_info.data.borrow_mut();
    let proof = Proof::try_from_bytes_mut(&mut proof_data)?;
    pool.last_total_members = pool.total_members;
    let previous_balance = proof.balance;
    drop(proof_data);

    // Submit solution to the ORE program
    let solution = Solution::new(args.digest, args.nonce);
    let mine_accounts = &[
        signer.clone(),
        bus_info.clone(),
        config_info.clone(),
        proof_info.clone(),
        instructions_sysvar.clone(),
        slot_hashes_sysvar.clone(),
    ];
    let mine_accounts = [mine_accounts, optional_accounts].concat();
    let optional_accounts = optional_accounts.iter().map(|a| *a.key).collect();
    solana_program::program::invoke(
        &ore_api::sdk::mine(
            *signer.key,
            *pool_info.key,
            *bus_info.key,
            solution,
            optional_accounts,
        ),
        &mine_accounts,
    )?;

    // Parse the proof balance again
    // to compute the diff which gives us the reward for attribution.
    let mut proof_data = proof_info.data.borrow_mut();
    let proof = Proof::try_from_bytes_mut(&mut proof_data)?;
    let new_balance = proof.balance;
    let reward = new_balance.saturating_sub(previous_balance);
    pool.reward = reward;
    pool.last_hash_at = proof.last_hash_at;

    Ok(())
}
