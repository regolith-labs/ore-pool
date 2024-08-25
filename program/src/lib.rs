mod attribute;
mod claim;
mod launch;
mod open;
mod submit;

use attribute::*;
use claim::*;
use launch::*;
use open::*;
use submit::*;

use ore_pool_api::instruction::PoolInstruction;
use solana_program::{
    self, account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

solana_program::entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if program_id.ne(&ore_pool_api::id()) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let (tag, data) = data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match PoolInstruction::try_from(*tag).or(Err(ProgramError::InvalidInstructionData))? {
        // User
        PoolInstruction::Open => process_open(accounts, data)?,
        PoolInstruction::Claim => process_claim(accounts, data)?,

        // Admin
        PoolInstruction::Attribute => process_attribute(accounts, data)?,
        PoolInstruction::Launch => process_launch(accounts, data)?,
        PoolInstruction::Submit => process_submit(accounts, data)?,
    }

    Ok(())
}
