use ore_pool_api::instruction::Attribute;
use solana_sdk::{program_error::ProgramError, transaction::Transaction};

use crate::error::Error;

pub fn validate_attribution(transaction: &Transaction, total_balance: i64) -> Result<(), Error> {
    let instructions = &transaction.message.instructions;
    // validate that all but the last instruction are compute budget
    let n = instructions.len();
    let num_compute_budget_instructions = n - 1;
    if num_compute_budget_instructions <= 1 {
        return Err(Error::Internal(
            "attribution transactions must contain at least two compute budget instructions"
                .to_string(),
        ));
    }
    let compute_budget_instructions = &instructions[..num_compute_budget_instructions];
    for ix in compute_budget_instructions {
        let program_id = transaction
            .message
            .account_keys
            .get(ix.program_id_index as usize)
            .ok_or(Error::Internal("missing program id".to_string()))?;
        if program_id.ne(&solana_sdk::compute_budget::id()) {
            return Err(Error::Internal(
                "expected instruction to be compute budget".to_string(),
            ));
        }
    }
    // validate that last instruction is the attribution
    let last = instructions
        .last()
        .ok_or(Error::Internal("empty attribution transaction".to_string()))?;
    let last_program_id = transaction
        .message
        .account_keys
        .get(last.program_id_index as usize)
        .ok_or(Error::Internal("missing program id".to_string()))?;
    if last_program_id.ne(&ore_pool_api::id()) {
        return Err(Error::Internal(
            "expected instruction to be pool program".to_string(),
        ));
    }
    // validate attribution amount
    let data = last.data.as_slice();
    let (_tag, data) = data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;
    let args = Attribute::try_from_bytes(data)?;
    let args_total_balance = u64::from_le_bytes(args.total_balance);
    if args_total_balance.ne(&(total_balance as u64)) {
        return Err(Error::Internal("invalid total balance arg".to_string()));
    }
    Ok(())
}
