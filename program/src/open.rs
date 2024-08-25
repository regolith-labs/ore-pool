use std::mem::size_of;

use ore_pool_api::{
    consts::*,
    instruction::*,
    loaders::*,
    state::{Member, Pool},
};
use ore_utils::{create_pda, loaders::*, AccountDeserialize, Discriminator};
use solana_program::{
    self, account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    system_program,
};

/// Open creates a new account for a pool participant.
pub fn process_open<'a, 'info>(accounts: &'a [AccountInfo<'info>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = OpenArgs::try_from_bytes(data)?;

    // Load accounts.
    let [signer, member_info, pool_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_signer(signer)?;
    load_uninitialized_pda(
        pool_info,
        &[MEMBER, signer.key.as_ref()],
        args.member_bump,
        &ore_pool_api::id(),
    )?;
    load_pool(pool_info, true)?;
    load_program(system_program, system_program::id())?;

    // Initialize member account
    create_pda(
        member_info,
        &ore_pool_api::id(),
        8 + size_of::<Member>(),
        &[MEMBER, signer.key.as_ref(), &[args.member_bump]],
        system_program,
        signer,
    )?;
    let mut member_data = member_info.try_borrow_mut_data()?;
    member_data[0] = Member::discriminator() as u8;
    let member = Member::try_from_bytes_mut(&mut member_data)?;
    member.authority = *signer.key;
    member.balance = 0;

    // Update member count
    let mut pool_data = pool_info.try_borrow_mut_data()?;
    let pool = Pool::try_from_bytes_mut(&mut pool_data)?;
    pool.total_members = pool.total_members.checked_add(1).unwrap();

    Ok(())
}
