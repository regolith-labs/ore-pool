use num_enum::IntoPrimitive;
use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq, IntoPrimitive)]
#[repr(u32)]
pub enum PoolError {
    #[error("Dummy error")]
    Dummy = 0,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("operator server url must be 128 bytes or less")]
    UrlTooLarge,
}

impl From<PoolError> for ProgramError {
    fn from(e: PoolError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
