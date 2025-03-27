use ore_pool_api::{instruction::Attribute, prelude::PoolInstruction};
use solana_sdk::{program_error::ProgramError, transaction::Transaction};

use crate::error::Error;

pub fn validate_attribution(transaction: &Transaction, total_balance: i64) -> Result<(), Error> {
    let instructions = &transaction.message.instructions;
    let n = instructions.len();

    // Transaction must have at least one instruction
    if n < 1 {
        return Err(Error::Internal(
            "transaction must contain at least one instruction".to_string(),
        ));
    }

    // Find the index of the first non-compute budget instruction
    let mut first_non_compute_budget_idx = 0;
    while first_non_compute_budget_idx < n {
        let ix = &instructions[first_non_compute_budget_idx];
        let program_id = transaction
            .message
            .account_keys
            .get(ix.program_id_index as usize)
            .ok_or(Error::Internal("missing program id".to_string()))?;

        if program_id.ne(&solana_sdk::compute_budget::id()) {
            break;
        }

        first_non_compute_budget_idx += 1;
    }

    // After compute budget instructions, we need at least one instruction (attribution)
    // and at most two instructions (attribution and claim)
    let remaining_instructions = n - first_non_compute_budget_idx;
    if remaining_instructions < 1 || remaining_instructions > 2 {
        return Err(Error::Internal(
            "after compute budget instructions, transaction must contain at least one and at most two instructions"
                .to_string(),
        ));
    }

    // Validate the first non-compute budget instruction as an ore pool attribution instruction
    let attr_idx = first_non_compute_budget_idx;
    let attr_ix = &instructions[attr_idx];
    let attr_program_id = transaction
        .message
        .account_keys
        .get(attr_ix.program_id_index as usize)
        .ok_or(Error::Internal(
            "missing program id for attribution instruction".to_string(),
        ))?;

    if attr_program_id.ne(&ore_pool_api::id()) {
        return Err(Error::Internal(
            "first non-compute budget instruction must be an ore_pool instruction".to_string(),
        ));
    }

    // Validate that it's specifically an attribution instruction
    let attr_data = attr_ix.data.as_slice();
    let (attr_tag, attr_data) = attr_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;
    let attr_tag =
        PoolInstruction::try_from(*attr_tag).or(Err(ProgramError::InvalidInstructionData))?;
    if attr_tag.ne(&PoolInstruction::Attribute) {
        return Err(Error::Internal(
            "first non-compute budget instruction must be an attribution instruction".to_string(),
        ));
    }

    // Validate attribution amount
    let args = Attribute::try_from_bytes(attr_data)?;
    let args_total_balance = u64::from_le_bytes(args.total_balance);
    if args_total_balance.ne(&(total_balance as u64)) {
        return Err(Error::Internal("invalid total balance arg".to_string()));
    }

    // If there's a second non-compute budget instruction, validate it as a claim instruction
    if remaining_instructions == 2 {
        let claim_idx = first_non_compute_budget_idx + 1;
        let claim_ix = &instructions[claim_idx];
        let claim_program_id = transaction
            .message
            .account_keys
            .get(claim_ix.program_id_index as usize)
            .ok_or(Error::Internal(
                "missing program id for claim instruction".to_string(),
            ))?;

        if claim_program_id.ne(&ore_pool_api::id()) {
            return Err(Error::Internal(
                "second non-compute budget instruction must be an ore_pool instruction".to_string(),
            ));
        }

        // Validate as specifically a claim instruction
        let claim_data = claim_ix.data.as_slice();
        let (claim_tag, _claim_data) = claim_data
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;
        let claim_tag =
            PoolInstruction::try_from(*claim_tag).or(Err(ProgramError::InvalidInstructionData))?;
        if claim_tag.ne(&PoolInstruction::Claim) {
            return Err(Error::Internal(
                "second non-compute budget instruction must be a claim instruction".to_string(),
            ));
        }
    }

    Ok(())
}
