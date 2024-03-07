use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    instruction::UpdateDifficultyArgs, loaders::*, state::Treasury, utils::AccountDeserialize,
};

/// UpdateDifficulty updates the program's global difficulty value. It has 1 responsibility:
/// 1. Update the difficulty.
///
/// Safety requirements:
/// - Can only succeed if the signer is the current program admin.
/// - Can only succeed if the provided treasury is valid.
pub fn process_update_difficulty<'a, 'info>(
    _program_id: &Pubkey,
    accounts: &'a [AccountInfo<'info>],
    data: &[u8],
) -> ProgramResult {
    // Parse args
    let args = UpdateDifficultyArgs::try_from_bytes(data)?;

    // Load accounts
    let [signer, treasury_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_signer(signer)?;
    load_treasury(treasury_info, true)?;

    // Validate admin signer
    let mut treasury_data = treasury_info.data.borrow_mut();
    let treasury = Treasury::try_from_bytes_mut(&mut treasury_data)?;
    if treasury.admin.ne(&signer.key) {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Update admin
    treasury.difficulty = args.new_difficulty;

    Ok(())
}