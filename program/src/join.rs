use std::mem::size_of;

use ore_pool_api::{
    consts::*,
    instruction::*,
    loaders::*,
    state::{Member, Pool},
};
use ore_utils::{
    create_pda, load_program, load_signer, load_system_account, load_uninitialized_pda,
    AccountDeserialize, Discriminator,
};
use solana_program::{
    self, account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    system_program,
};

/// Join creates a new account for a pool participant.
pub fn process_join(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = Join::try_from_bytes(data)?;

    // Load accounts.
    let [signer, member_authority_info, member_info, pool_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_signer(signer)?;
    load_system_account(member_authority_info, false)?;
    load_uninitialized_pda(
        member_info,
        &[
            MEMBER,
            member_authority_info.key.as_ref(),
            pool_info.key.as_ref(),
        ],
        args.member_bump,
        &ore_pool_api::id(),
    )?;
    load_any_pool(pool_info, true)?;
    load_program(system_program, system_program::id())?;

    // Initialize member account
    create_pda(
        member_info,
        &ore_pool_api::id(),
        8 + size_of::<Member>(),
        &[
            MEMBER,
            member_authority_info.key.as_ref(),
            pool_info.key.as_ref(),
            &[args.member_bump],
        ],
        system_program,
        signer,
    )?;

    // Update member count
    let mut pool_data = pool_info.try_borrow_mut_data()?;
    let pool = Pool::try_from_bytes_mut(&mut pool_data)?;

    // Init member
    let mut member_data = member_info.try_borrow_mut_data()?;
    member_data[0] = Member::discriminator();
    let member = Member::try_from_bytes_mut(&mut member_data)?;
    member.authority = *member_authority_info.key;
    member.balance = 0;
    member.total_balance = 0;
    member.pool = *pool_info.key;
    member.id = pool.total_members; // zero index
    pool.total_members = pool.total_members.checked_add(1).unwrap();

    Ok(())
}
