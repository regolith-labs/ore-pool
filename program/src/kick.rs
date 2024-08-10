use ore_api::*;
use ore_pool_api::{consts::*, instruction::*, loaders::*};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    {self},
};

/// Kick ...
pub fn process_kick<'a, 'info>(accounts: &'a [AccountInfo<'info>], data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer, member_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_operator(signer)?;
    load_any_member(member_info, true)?;

    // Reject kicked pool members
    let mut member_data = member_info.try_borrow_mut_data()?;
    let member = Member::try_from_bytes_mut(&mut member_data)?;
    member.is_kicked = 1;

    Ok(())
}
