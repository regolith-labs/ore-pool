use ore_pool_api::prelude::*;
use steel::*;

pub fn process_migrate_quick(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = QuickMigrate::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    // Load accounts.
    let [signer_info, pool_info, migration_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?.has_address(&ADMIN_ADDRESS)?;
    let pool = pool_info.as_account_mut::<Pool>(&ore_pool_api::ID)?;
    migration_info
        .as_account_mut::<Migration>(&ore_pool_api::ID)?
        .assert_mut(|m| m.pool == *pool_info.key)?;
    system_program.is_program(&system_program::ID)?;

    // Reset total rewards
    pool.total_rewards = amount;

    // Create migration account
    migration_info.close(signer_info)?;

    Ok(())
}
