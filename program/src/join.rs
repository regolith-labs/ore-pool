use ore_pool_api::prelude::*;
use steel::*;

/// Join creates a new account for a pool participant.
pub fn process_join(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = Join::try_from_bytes(data)?;

    // Load accounts.
    let [signer_info, member_authority_info, member_info, pool_info, system_program] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    member_info.is_empty()?.is_writable()?.has_seeds(
        &[
            MEMBER,
            member_authority_info.key.as_ref(),
            pool_info.key.as_ref(),
        ],
        args.member_bump,
        &ore_pool_api::ID,
    )?;
    let pool = pool_info.to_account_mut::<Pool>(&ore_pool_api::ID)?;
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
        signer_info,
    )?;

    // Init member
    let member = member_info.to_account_mut::<Member>(&ore_pool_api::ID)?;
    member.authority = *member_authority_info.key;
    member.balance = 0;
    member.total_balance = 0;
    member.pool = *pool_info.key;
    member.id = pool.total_members; // zero index

    // Update total pool member count.
    pool.total_members = pool.total_members.checked_add(1).unwrap();

    Ok(())
}
