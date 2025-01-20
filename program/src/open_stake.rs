use ore_pool_api::prelude::*;
use steel::*;

/// Opens a new stake account for the pool in the boost program.
pub fn process_open_stake(_accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    Err(PoolError::WithdrawOnlyMode.into())
}
