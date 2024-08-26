use std::env::VarError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("std io")]
    StdIO(#[from] std::io::Error),
    #[error("std env")]
    StdEnv(#[from] VarError),
    #[error("solana client")]
    SolanaClient(#[from] solana_client::client_error::ClientError),
    #[error("solana program")]
    SolanaProgram(#[from] solana_sdk::program_error::ProgramError),
    #[error("{0}")]
    Internal(String),
}
