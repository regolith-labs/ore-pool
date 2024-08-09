use ore_pool_api::instruction::*;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    {self},
};

/// Initialize sets up the ORE program to begin mining.
pub fn process_initialize<'a, 'info>(
    accounts: &'a [AccountInfo<'info>],
    data: &[u8],
) -> ProgramResult {
    // Parse args.
    let args = InitializeArgs::try_from_bytes(data)?;

    // Load accounts.
    // let [signer] =
    //     accounts
    // else {
    //     return Err(ProgramError::NotEnoughAccountKeys);
    // };
    // load_signer(signer)?;

    Ok(())
}
