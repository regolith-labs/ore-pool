mod attribute;
mod claim;
mod commit;
mod join;
mod launch;
mod open_share;
mod open_stake;
mod stake;
mod submit;
mod unstake;

use attribute::*;
use claim::*;
use commit::*;
use join::*;
use launch::*;
use open_share::*;
use open_stake::*;
use stake::*;
use submit::*;
use unstake::*;

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
        PoolInstruction::Join => process_join(accounts, data)?,
        PoolInstruction::Claim => process_claim(accounts, data)?,
        PoolInstruction::OpenShare => process_open_share(accounts, data)?,
        PoolInstruction::Stake => process_stake(accounts, data)?,
        PoolInstruction::Unstake => process_unstake(accounts, data)?,

        // Admin
        PoolInstruction::Attribute => process_attribute(accounts, data)?,
        PoolInstruction::Commit => process_commit(accounts, data)?,
        PoolInstruction::Launch => process_launch(accounts, data)?,
        PoolInstruction::OpenStake => process_open_stake(accounts, data)?,
        PoolInstruction::Submit => process_submit(accounts, data)?,
    }

    Ok(())
}
