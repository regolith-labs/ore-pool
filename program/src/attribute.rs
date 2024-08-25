use ore_pool_api::{instruction::*, loaders::*, state::Member};
use ore_utils::AccountDeserialize;
use solana_program::{
    self, account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
};

/// Attribute ...
pub fn process_attribute<'a, 'info>(
    accounts: &'a [AccountInfo<'info>],
    data: &[u8],
) -> ProgramResult {
    // Parse args.
    let args = AttributeArgs::try_from_bytes(data)?;

    // Load accounts.
    let [signer, member_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_operator(signer)?;
    load_any_member(member_info, false)?;

    // Reject kicked pool members
    let member_data = member_info.data.borrow();
    let member = Member::try_from_bytes(&member_data)?;

    // TODO

    Ok(())
}
