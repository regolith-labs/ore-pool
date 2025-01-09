use ore_pool_api::prelude::*;
use steel::*;

pub fn process_migrate_member_balance(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer_info, pool_info, member_info, migration_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?.has_address(&ADMIN_ADDRESS)?;
    let pool = pool_info.as_account_mut::<Pool>(&ore_pool_api::ID)?;
    let member = member_info
        .as_account::<Member>(&ore_pool_api::ID)?
        .assert(|m| m.pool == *pool_info.key)?;
    let migration = migration_info
        .as_account_mut::<Migration>(&ore_pool_api::ID)?
        .assert_mut(|m| m.pool == *pool_info.key)?
        .assert_mut(|m| m.members_migrated == member.id)?;
    system_program.is_program(&system_program::ID)?;

    // Increment pool total rewards counter
    pool.total_rewards += member.balance;

    // Increment migrated balance
    migration.members_migrated += 1;

    // End migration if done,
    if migration.members_migrated == pool.total_members {
        migration_info.close(signer_info)?;
    }

    Ok(())
}