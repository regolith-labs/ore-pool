use ore_pool_api::prelude::*;
use steel::*;

pub fn process_migrate_pool(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer_info, pool_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?.has_address(&ADMIN_ADDRESS)?;
    pool_info.as_account_mut::<Pool>(&ore_pool_api::ID)?;

    // Allocate space for new data field.
    pool_info.realloc(pool_info.data.borrow().len() + 8, true)?;

    Ok(())
}
