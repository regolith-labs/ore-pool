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

    // Update pool submissions count
    pool.total_submissions = pool.total_submissions.checked_add(1).unwrap();

    // And the attestation of observed hash-power
    pool.attestation = args.attestation;

    // Parse the proof balance before submitting solution
    // as previous balance to compute reward.
    pool.last_total_members = pool.total_members;
    let previous_balance = proof.balance;

    // Build instruction for submitting solution to the ORE program
    let solution = Solution::new(args.digest, args.nonce);
    let mut boost_keys = None;
    let mut mine_accounts = vec![
        signer_info.clone(),
        bus_info.clone(),
        config_info.clone(),
        proof_info.clone(),
        instructions_sysvar.clone(),
        slot_hashes_sysvar.clone(),
    ];
    if let [boost_info, _boost_proof_info, reservation_info] = boost_accounts {
        boost_keys = Some([*boost_info.key, *reservation_info.key]);
        mine_accounts = [mine_accounts, boost_accounts.to_vec()].concat();
    }

    // Invoke CPI
    solana_program::program::invoke(
        &ore_api::sdk::mine(
            *signer_info.key,
            *pool_info.key,
            *bus_info.key,
            solution,
            boost_keys,
        ),
        &mine_accounts,
    )?;

    // Parse the proof balance again
    // to compute the diff which gives us the reward for attribution.
    let new_balance = proof.balance;
    let reward = new_balance.saturating_sub(previous_balance);
    pool.reward = reward;
    pool.last_hash_at = proof.last_hash_at;

    Ok(())
}
