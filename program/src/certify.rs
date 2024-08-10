use ore_api::*;
use ore_pool_api::{consts::*, instruction::*, loaders::*};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    {self},
};

/// Certify allows the pool operator to authenticate the validity of hashes considered in a given batch.
///
/// This process generates a certification which must match the attestation provided by the operator when the 
/// pool's best hash was submitted for mining. If the attestation cannot be certified, then the mining rewards in this
/// bathc are left unclaimable.
///
/// SECURITY
/// The pool operator is responsible attributing hashes to pool members. This discretion creates an opportunity for a malicious 
/// pool operator to "steal" hashes from participating pool members. That is, a member could submit a valid hash to the operator, 
/// and the operator could attribute it another member. No smart-contract can detect or prevent this type of fraud. Ultimately the 
/// pool operator is providing a service to the participating members and such fraud is detectable by the members. The only remediation 
/// for such fraud is to simply stop participating in the pool. The certification process in this contract aims only to prove the 
/// authenticity of participating hashpower in the pool (that is, that hashpower is not fake). It does not prove the identity of the 
/// hashpower contributor.
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
    load_operator(signer)?;
    load_any_batch(batch_info, true)?;
    load_any_member(member_info, false)?;

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
        batch.best_nonce = nonce;
    }

    // Reject kicked pool members
    let member_data = member_info.data.borrow();
    let member = Member::try_from_bytes(&member_data)?;
    if member.is_kicked.gt(&0) {
        return Err(PoolError::Dummy.into());
    }

    // TODO Extend zero copy account with member balance

    // Update the batch metadata
    batch.total_solutions = batch.total_solutions.checked_add(1).unwrap();
    batch.total_difficulty_score = batch.total_difficulty_score.checked_add(difficulty_score).unwrap();

    // Update the batch certification
    batch.certification = hashv(&[
        batch.certification.as_slice(),
        args.nonce.as_slice(),
        member_info.key.ref(),
        batch.best_nonce.to_le_bytes().as_slice(),
        batch.total_solutions.to_le_bytes().as_slice(),
        batch.total_difficulty_score.to_le_bytes().as_slice(),
    ]);
    
    Ok(())
}
