use drillx::Solution;
use ore_api::{
    event::MineEvent,
    loaders::{load_any_bus, load_config, load_proof},
    state::Proof,
};
use ore_pool_api::{error::PoolError, instruction::*, loaders::*, state::Pool};
use ore_utils::{load_program, load_signer, load_sysvar, AccountDeserialize};
use solana_program::{
    self,
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    log::{self, sol_log},
    program::{get_return_data, set_return_data},
    program_error::ProgramError,
    system_program, sysvar,
};

/// Submit sends the pool's best hash to the ORE mining contract.
pub fn process_submit(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = Submit::try_from_bytes(data)?;

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

    // Update pool submissions count
    let mut pool_data = pool_info.data.borrow_mut();
    let pool = Pool::try_from_bytes_mut(&mut pool_data)?;
    pool.total_submissions = pool.total_submissions.checked_add(1).unwrap();
    // And the attestation of observed hash-power
    pool.attestation = args.attestation;

    // Update the last hash to align with the proof that we are currently solving for.
    // can think of this is a foreign key join.
    // the idea is that the state of this account will/can be indexed,
    // and later referenced or played back for historical information.
    let mut proof_data = proof_info.data.borrow_mut();
    let proof = Proof::try_from_bytes_mut(&mut proof_data)?;
    pool.last_hash_at = proof.last_hash_at;
    pool.last_total_members = pool.total_members;
    drop(proof_data);

    // Submit solution to the ORE program
    let solution = Solution::new(args.digest, args.nonce);
    solana_program::program::invoke(
        &ore_api::instruction::mine(*signer.key, *pool_info.key, *bus_info.key, solution),
        &[
            signer.clone(),
            bus_info.clone(),
            config_info.clone(),
            proof_info.clone(),
            instructions_sysvar.clone(),
            slot_hashes_sysvar.clone(),
        ],
    )?;

    // Parse reward from return data
    let (pubkey, reward_bytes) = get_return_data().ok_or(PoolError::MissingMiningReward)?;
    log::sol_log(format!("pubkey: {:?}", pubkey).as_str());
    log::sol_log(format!("bytes: {:?}", reward_bytes.as_slice()).as_str());
    let reward = *MineEvent::from_bytes(reward_bytes.as_slice());
    log::sol_log(format!("reward: {:?}", reward).as_str());
    // let reward: MineEvent = *bytemuck::try_from_bytes(reward_bytes.as_slice())
    //     .map_err(|_| PoolError::CouldNotParseMiningReward)?;

    // Write rewards back to return data to parse from client
    set_return_data(reward.to_bytes());

    Ok(())
}
