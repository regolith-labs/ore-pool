use ore_api::*;
use ore_pool_api::{consts::*, instruction::*, loaders::*};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    {self},
};

/// Certify ...
pub fn process_certify<'a, 'info>(
    accounts: &'a [AccountInfo<'info>],
    data: &[u8],
) -> ProgramResult {
    // Parse args.
    let args = CertifyArgs::try_from_bytes(data)?;

    // Load accounts.
    let [signer, batch_info, member_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    // TODO Account loaders
    // load_operator(signer)?;
    // load_signer(miner_info)?;
    // load_program(ore_program, ore_api::id())?;
    // load_program(system_program, system_program::id())?;
    // load_sysvar(instructions_sysvar, sysvar::instructions::id())?;
    // load_sysvar(slot_hashes_sysvar, sysvar::slot_hashes::id())?;

    // TODO If batch is already certified, then exit
    let mut batch_data = batch_info.data.borrow_mut();
    let batch = Batch::try_from_bytes_mut(&mut batch_data)?;
    if batch.attestation.eq(&batch.certification) {
        return Err(PoolError::Dummy.into());
    }

    // Verify the solution
    let solution = Solution::new(args.digest, args.nonce);
    if !solution.is_valid(&batch.challenge) {
        return Err(PoolError::Dummy.into());
    }

    // Get hash difficulty score 
    let hash = solution.to_hash();
    let difficulty = hash.difficulty();
    let difficulty_score = 2u128.pow(difficulty);
    if difficulty.gt(&batch.best_difficulty) {
        batch.best_difficulty = difficulty;
        batch.best_digest = digest;
        batch.best_nonce = nonce;
    }

    // Reject kicked pool members
    let member_data = member_info.data.borrow();
    let member = Member::try_from_bytes(&member_data)?;
    if member.is_kicked.gt(&0) {
        return Err(PoolError::Dummy.into());
    }

    // TODO Verify the signature

    // TODO Extend zero copy account with member balance

    // Update the batch metadata
    batch.total_solutions = batch.total_solutions.checked_add(1).unwrap();
    batch.total_difficulty_score = batch.total_difficulty_score.checked_add(difficulty_score).unwrap();

    // Update the batch certification
    batch.certification = hashv(&[
        batch.certification.as_slice(),
        member.authority.ref(),
        args.digest.as_slice(),
        args.nonce.as_slice(),
        batch.best_digest.to_le_bytes().as_slice(),
        batch.best_nonce.to_le_bytes().as_slice(),
        batch.total_solutions.to_le_bytes().as_slice(),
        batch.total_difficulty_score.to_le_bytes().as_slice(),
    ]);
    
    Ok(())
}
