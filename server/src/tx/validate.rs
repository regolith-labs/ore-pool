use crate::tx::submit::JITO_TIP_ADDRESSES;
use ore_pool_api::{instruction::Attribute, prelude::PoolInstruction};
use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{
    program_error::ProgramError, system_instruction, system_program, transaction::Transaction,
};
use spl_associated_token_account::ID as SPL_ASSOCIATED_TOKEN_ID;

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
    if args_total_balance.gt(&(total_balance as u64)) {
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

    // Check for subsequent instructions after the attribution instruction
    let mut ore_pool_end_idx = first_non_allowed_prefix_idx + 1;

    // Check for an spl-associated-token-account instruction that might come before the claim
    if ore_pool_end_idx < n {
        let next_ix = &instructions[ore_pool_end_idx];
        let next_program_id = transaction
            .message
            .account_keys
            .get(next_ix.program_id_index as usize)
            .ok_or(Error::Internal(
                "missing program id for instruction after attribution".to_string(),
            ))?;

        // If the next instruction is from spl-associated-token-account, advance the index
        if next_program_id.eq(&SPL_ASSOCIATED_TOKEN_ID) {
            log::info!("Found spl-associated-token-account instruction");
            ore_pool_end_idx += 1;
        }
    }

    // Check for a claim instruction (which may come after spl-associated-token-account)
    if ore_pool_end_idx < n {
        let claim_ix = &instructions[ore_pool_end_idx];
        let claim_program_id = transaction
            .message
            .account_keys
            .get(claim_ix.program_id_index as usize)
            .ok_or(Error::Internal(
                "missing program id for potential claim instruction".to_string(),
            ))?;

        // If the instruction is from ore_pool, validate it as a claim instruction
        if claim_program_id.eq(&ore_pool_api::id()) {
            // Validate as specifically a claim instruction
            let claim_data = claim_ix.data.as_slice();
            let (claim_tag, _claim_data) = claim_data
                .split_first()
                .ok_or(ProgramError::InvalidInstructionData)?;
            let claim_tag = PoolInstruction::try_from(*claim_tag)
                .or(Err(ProgramError::InvalidInstructionData))?;
            if claim_tag.ne(&PoolInstruction::Claim) {
                return Err(Error::Internal(
                    "ore_pool instruction after attribution must be a claim instruction"
                        .to_string(),
                ));
            }

            ore_pool_end_idx += 1;
        }
    }

    // Validate any remaining instructions belong to lighthouse program or are Jito tips
    for i in ore_pool_end_idx..n {
        let ix = &instructions[i];
        let program_id_index = ix.program_id_index as usize;
        let program_id = transaction
            .message
            .account_keys
            .get(program_id_index)
            .ok_or(Error::Internal(
                format!("missing program id for instruction at index {}", i).to_string(),
            ))?;

        // Allow lighthouse instructions
        if program_id.eq(&LH_PUBKEY) {
            continue;
        }

        // Allow system program transfer to Jito tip addresses
        if program_id.eq(&system_program::id()) {
            // Check if it's a transfer instruction by attempting deserialization
            if let Ok(system_instruction::SystemInstruction::Transfer { lamports: _ }) =
                bincode::deserialize(&ix.data)
            {
                // Get the destination account index (second account for transfer)
                if let Some(to_account_index) = ix.accounts.get(1) {
                    // Get the destination pubkey from the transaction message's account keys
                    if let Some(to_pubkey) = transaction
                        .message
                        .account_keys
                        .get(*to_account_index as usize)
                    {
                        // Check if the destination is a known Jito tip address
                        if JITO_TIP_ADDRESSES.contains(to_pubkey) {
                            log::debug!("Allowing Jito tip transfer instruction at index {}", i);
                            continue; // It's a valid Jito tip, allow it
                        }
                    }
                }
            }
        }

        // If it's neither lighthouse nor a valid Jito tip transfer, return error
        return Err(Error::Internal(
            format!(
                "instruction at index {} must belong to lighthouse program or be a Jito tip transfer. Found program id: {}",
                i, program_id
            )
            .to_string(),
        ));
    }

    Ok(())
}
