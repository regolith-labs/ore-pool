mod attribute;
mod claim;
mod commit;
mod join;
mod launch;
mod open_share;
mod open_share_rewards;
mod open_stake;
mod open_total_rewards;
mod stake;
mod submit;
mod unstake;

use attribute::*;
use claim::*;
use commit::*;
use join::*;
use launch::*;
use open_share::*;
use open_share_rewards::process_open_share_rewards;
use open_stake::*;
use open_total_rewards::process_open_total_rewards;
use stake::*;
use submit::*;
use unstake::*;

use ore_pool_api::prelude::*;
use steel::*;

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
        PoolInstruction::OpenTotalRewards => process_open_total_rewards(accounts, data)?,
        PoolInstruction::OpenShareRewards => process_open_share_rewards(accounts, data)?,
    }

    Ok(())
}

entrypoint!(process_instruction);
