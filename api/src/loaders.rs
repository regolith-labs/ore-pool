use steel::*;

use crate::state::{Member, Pool, Share};

/// Errors if:
/// - Owner is not pool program.
/// - Data is empty.
/// - Data cannot be parsed to a member account.
/// - Member authority is not expected value.
/// - Expected to be writable, but is not.
pub fn load_member(
    info: &AccountInfo<'_>,
    authority: &Pubkey,
    pool: &Pubkey,
    is_writable: bool,
) -> Result<(), ProgramError> {
    if info.owner.ne(&crate::id()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    let member_data = info.data.borrow();
    let member = Member::try_from_bytes(&member_data)?;

    if member.authority.ne(authority) {
        return Err(ProgramError::InvalidAccountData);
    }

    if member.pool.ne(pool) {
        return Err(ProgramError::InvalidAccountData);
    }

    if is_writable && !info.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

/// Errors if:
/// - Owner is not pool program.
/// - Data is empty.
/// - Account discriminator does not match expected value.
/// - Expected to be writable, but is not.
pub fn load_any_member(
    info: &AccountInfo<'_>,
    pool: &Pubkey,
    is_writable: bool,
) -> Result<(), ProgramError> {
    if info.owner.ne(&crate::id()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    let member_data = info.data.borrow();
    let member = Member::try_from_bytes(&member_data)?;

    if member.pool.ne(pool) {
        return Err(ProgramError::InvalidAccountData);
    }

    if is_writable && !info.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

/// Errors if:
/// - Owner is not pool program.
/// - Data is empty.
/// - Data cannot be deserialized into a pool account.
/// - Pool authority is not expected value.
/// - Expected to be writable, but is not.
pub fn load_pool(
    info: &AccountInfo<'_>,
    authority: &Pubkey,
    is_writable: bool,
) -> Result<(), ProgramError> {
    if info.owner.ne(&crate::id()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    let pool_data = info.data.borrow();
    let pool = Pool::try_from_bytes(&pool_data)?;

    if pool.authority.ne(authority) {
        return Err(ProgramError::InvalidAccountData);
    }

    if is_writable && !info.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

/// Errors if:
/// - Owner is not pool program.
/// - Data is empty.
/// - Account discriminator does not match expected value.
/// - Expected to be writable, but is not.
pub fn load_any_pool(info: &AccountInfo<'_>, is_writable: bool) -> Result<(), ProgramError> {
    if info.owner.ne(&crate::id()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    if info.data.borrow()[0].ne(&(Pool::discriminator())) {
        return Err(solana_program::program_error::ProgramError::InvalidAccountData);
    }

    if is_writable && !info.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

/// Errors if:
/// - Owner is not pool program.
/// - Data is empty.
/// - Data cannot be deserialized into a share account.
/// - Share authority is not expected value.
/// - Share mint account is not the expected value.
/// - Expected to be writable, but is not.
pub fn load_share(
    info: &AccountInfo<'_>,
    authority: &Pubkey,
    pool: &Pubkey,
    mint: &Pubkey,
    is_writable: bool,
) -> Result<(), ProgramError> {
    if info.owner.ne(&crate::id()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    let share_data = info.data.borrow();
    let share = Share::try_from_bytes(&share_data)?;

    if share.authority.ne(authority) {
        return Err(ProgramError::InvalidAccountData);
    }

    if share.pool.ne(pool) {
        return Err(ProgramError::InvalidAccountData);
    }

    if share.mint.ne(mint) {
        return Err(ProgramError::InvalidAccountData);
    }

    if is_writable && !info.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}
