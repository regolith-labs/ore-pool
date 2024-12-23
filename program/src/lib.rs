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

use ore_pool_api::prelude::*;
use steel::*;

#[allow(deprecated)]
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let (ix, data) = parse_instruction(&ore_pool_api::ID, program_id, data)?;
    match ix {
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

entrypoint!(process_instruction);
