use ore_pool_api::prelude::*;
use steel::*;

pub fn process_migrate_pool(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer_info, pool_info, migration_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?.has_address(&ADMIN_ADDRESS)?;
    pool_info.as_account_mut::<Pool>(&ore_pool_api::ID)?;
    migration_info
        .is_empty()?
        .is_writable()?
        .has_seeds(&[MIGRATION, pool_info.key.as_ref()], &ore_pool_api::ID)?;
    system_program.is_program(&system_program::ID)?;

    // Allocate space for new data field.
    pool_info.realloc(pool_info.data.borrow().len() + 8, true)?;

    // Create migration account
    create_account::<Migration>(
        migration_info,
        system_program,
        signer_info,
        &ore_pool_api::ID,
        &[MIGRATION, pool_info.key.as_ref()],
    )?;
    let migration = migration_info.as_account_mut::<Migration>(&ore_pool_api::ID)?;
    migration.pool = *pool_info.key;
    migration.members_migrated = 0;

    Ok(())
}
