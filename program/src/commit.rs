use ore_pool_api::prelude::*;
use steel::*;

/// Commit pending stake from the pool program into the boost program.
pub fn process_commit(_accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    Err(PoolError::WithdrawOnlyMode.into())
}
