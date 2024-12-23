use ore_pool_api::prelude::*;
use steel::*;

/// Opens a new share account for pool member to deposit stake.
pub fn process_open_share(_accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    Err(PoolError::WithdrawOnlyMode.into())
}
