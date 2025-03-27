use ore_pool_api::{instruction::Attribute, prelude::PoolInstruction};
use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{program_error::ProgramError, transaction::Transaction};

use crate::error::Error;

/// Lighthouse protocol pubkey,
/// web-browser wallets typically insert these instructions.
const LH_PUBKEY: Pubkey = pubkey!("L2TExMFKdjpN9kozasaurPirfHy9P8sbXoAN1qA3S95");

pub fn validate_attribution(
    transaction: &Transaction,
    member_authority: Pubkey,
    pool: Pubkey,
    total_balance: i64,
) -> Result<(), Error> {
    let instructions = &transaction.message.instructions;
    let n = instructions.len();

    // Transaction must have at least one instruction
    if n < 1 {
        return Err(Error::Internal(
            "transaction must contain at least one instruction".to_string(),
        ));
    }

    // Find the index of the first instruction that is neither compute budget nor lighthouse
    let mut first_non_allowed_prefix_idx = 0;
    while first_non_allowed_prefix_idx < n {
        let ix = &instructions[first_non_allowed_prefix_idx];
        let program_id = transaction
            .message
            .account_keys
            .get(ix.program_id_index as usize)
            .ok_or(Error::Internal("missing program id".to_string()))?;

        // Allow both compute budget and lighthouse instructions at the beginning
        if program_id.ne(&solana_sdk::compute_budget::id()) && program_id.ne(&LH_PUBKEY) {
            break;
        }

        first_non_allowed_prefix_idx += 1;
    }

    // After compute budget and lighthouse instructions, we need at least one instruction (attribution)
    let remaining_instructions = n - first_non_allowed_prefix_idx;
    if remaining_instructions < 1 {
        return Err(Error::Internal(
            "transaction must contain at least one instruction after compute budget and lighthouse instructions".to_string(),
        ));
    }

    // Validate the first non-compute budget/lighthouse instruction as an ore pool attribution instruction
    let attr_idx = first_non_allowed_prefix_idx;
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
            "first instruction after compute budget and lighthouse instructions must be an ore_pool instruction".to_string(),
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
            "first instruction after compute budget and lighthouse instructions must be an attribution instruction".to_string(),
        ));
    }

    // Validate attribution amount
    let args = Attribute::try_from_bytes(attr_data)?;
    let args_total_balance = u64::from_le_bytes(args.total_balance);
    if args_total_balance.ne(&(total_balance as u64)) {
        return Err(Error::Internal("invalid total balance arg".to_string()));
    }

    // Validate attribution member authority
    //
    // The fifth account in the attribution instruction is the member account
    let member_authority_index = attr_ix.accounts.get(4).ok_or(Error::MemberDoesNotExist)?;
    let member_parsed = transaction
        .message
        .account_keys
        .get((*member_authority_index) as usize)
        .ok_or(Error::MemberDoesNotExist)?;
    let (member_pda, _) = ore_pool_api::state::member_pda(member_authority, pool);
    if member_pda.ne(member_parsed) {
        return Err(Error::Internal(
            "payload and instruction member accounts do not match".to_string(),
        ));
    }

    // Check for a second ore_pool instruction (claim)
    let mut ore_pool_end_idx = first_non_allowed_prefix_idx + 1;

    if ore_pool_end_idx < n {
        let second_ix = &instructions[ore_pool_end_idx];
        let second_program_id = transaction
            .message
            .account_keys
            .get(second_ix.program_id_index as usize)
            .ok_or(Error::Internal(
                "missing program id for second instruction".to_string(),
            ))?;

        // If the second instruction is from ore_pool, validate it as a claim instruction
        if second_program_id.eq(&ore_pool_api::id()) {
            // Validate as specifically a claim instruction
            let claim_data = second_ix.data.as_slice();
            let (claim_tag, _claim_data) = claim_data
                .split_first()
                .ok_or(ProgramError::InvalidInstructionData)?;
            let claim_tag = PoolInstruction::try_from(*claim_tag)
                .or(Err(ProgramError::InvalidInstructionData))?;
            if claim_tag.ne(&PoolInstruction::Claim) {
                return Err(Error::Internal(
                    "second ore_pool instruction must be a claim instruction".to_string(),
                ));
            }

            ore_pool_end_idx += 1;
        }
    }

    // Validate any remaining instructions belong to lighthouse program
    for i in ore_pool_end_idx..n {
        let ix = &instructions[i];
        let program_id = transaction
            .message
            .account_keys
            .get(ix.program_id_index as usize)
            .ok_or(Error::Internal(
                format!("missing program id for instruction at index {}", i).to_string(),
            ))?;

        if program_id.ne(&LH_PUBKEY) {
            return Err(Error::Internal(
                format!(
                    "instruction at index {} must belong to lighthouse program",
                    i
                )
                .to_string(),
            ));
        }
    }

    Ok(())
}
