use ore_pool_api::prelude::*;
use steel::*;

/// Deposit tokens into a pool's pending stake account.
pub fn process_stake(_accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    Err(PoolError::WithdrawOnlyMode.into())
}
