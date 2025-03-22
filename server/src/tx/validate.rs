use ore_pool_api::{instruction::Attribute, prelude::PoolInstruction};
use solana_sdk::{program_error::ProgramError, transaction::Transaction};

use crate::error::Error;

pub fn validate_attribution(transaction: &Transaction, total_balance: i64) -> Result<(), Error> {
    let instructions = &transaction.message.instructions;

    // Validate transaction size
    // at least three instructions (just an attribution)
    // and no more than four (both an attribution and a claim)
    let n = instructions.len();
    if n < 3 || n > 4 {
        return Err(Error::Internal(
            "transaction must contain at least three and no more than four instructions"
                .to_string(),
        ));
    }

    // Check that the first two instructions are compute budget instructions
    for i in 0..2 {
        let ix = &instructions[i];
        let program_id = transaction
            .message
            .account_keys
            .get(ix.program_id_index as usize)
            .ok_or(Error::Internal("missing program id".to_string()))?;
        if program_id.ne(&solana_sdk::compute_budget::id()) {
            return Err(Error::Internal(
                "the first two instructions must be compute budget instructions".to_string(),
            ));
        }
    }

    // Validate as an ore pool instruction
    let third_ix = &instructions[2];
    let third_program_id = transaction
        .message
        .account_keys
        .get(third_ix.program_id_index as usize)
        .ok_or(Error::Internal(
            "missing program id for third instruction".to_string(),
        ))?;

    if third_program_id.ne(&ore_pool_api::id()) {
        return Err(Error::Internal(
            "third instruction must be an ore_pool instruction".to_string(),
        ));
    }

    // Validate that the third instruction is specifically an attribution instruction
    let third_data = third_ix.data.as_slice();
    let (third_tag, third_data) = third_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;
    let third_tag =
        PoolInstruction::try_from(*third_tag).or(Err(ProgramError::InvalidInstructionData))?;
    if third_tag.ne(&PoolInstruction::Attribute) {
        return Err(Error::Internal(
            "third instruction must be an attribution instruction".to_string(),
        ));
    }

    // Validate attribution amount
    let args = Attribute::try_from_bytes(third_data)?;
    let args_total_balance = u64::from_le_bytes(args.total_balance);
    if args_total_balance.ne(&(total_balance as u64)) {
        return Err(Error::Internal("invalid total balance arg".to_string()));
    }

    // If there are four instructions, validate that the fourth is a claim instruction
    if n == 4 {
        // Validate as an ore pool instruction
        let fourth_ix = &instructions[3];
        let fourth_program_id = transaction
            .message
            .account_keys
            .get(fourth_ix.program_id_index as usize)
            .ok_or(Error::Internal(
                "missing program id for fourth instruction".to_string(),
            ))?;

        if fourth_program_id.ne(&ore_pool_api::id()) {
            return Err(Error::Internal(
                "fourth instruction must be an ore_pool instruction".to_string(),
            ));
        }

        // Validate as specifically a claim instruction
        let fourth_data = fourth_ix.data.as_slice();
        let (fourth_tag, _fourth_data) = fourth_data
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;
        let fourth_tag =
            PoolInstruction::try_from(*fourth_tag).or(Err(ProgramError::InvalidInstructionData))?;
        if fourth_tag.ne(&PoolInstruction::Claim) {
            return Err(Error::Internal(
                "fourth instruction must be a claim instruction".to_string(),
            ));
        }
    }

    Ok(())
}
