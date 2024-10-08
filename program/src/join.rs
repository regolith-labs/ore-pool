use ore_pool_api::{
    consts::*,
    instruction::*,
    loaders::*,
    state::{Member, Pool},
};
use steel::*;

/// Join creates a new account for a pool participant.
pub fn process_join(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = Join::try_from_bytes(data)?;

    // Load accounts.
    let [signer, member_authority_info, member_info, pool_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer.is_signer()?;
    member_authority_info.is_empty()?.is_writable()?.has_seeds(
        &[
            MEMBER,
            member_authority_info.key.as_ref(),
            pool_info.key.as_ref(),
        ],
        args.member_bump,
        &ore_pool_api::ID,
    )?;
    load_any_pool(pool_info, true)?;
    system_program.is_program(&system_program::ID)?;

    // Initialize member account
    create_account::<Member>(
        member_info,
        &ore_pool_api::ID,
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
