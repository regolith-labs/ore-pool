use steel::*;

#[repr(u32)]
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq, IntoPrimitive)]
pub enum PoolError {
    #[error("Missing mining reward")]
    MissingMiningReward = 0,
    #[error("Could not parse mining reward")]
    CouldNotParseMiningReward = 1,
    #[error("Staking is in withdraw only mode")]
    WithdrawOnlyMode = 2,
    #[error("Cannot attribute more rewards than are currently claimable")]
    AttributionTooLarge = 3,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("operator server url must be 128 bytes or less")]
    UrlTooLarge,
}

error!(PoolError);
