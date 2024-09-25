use std::env::VarError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("std env")]
    StdEnv(#[from] VarError),
    #[error("could not ready keypair from provided path: {0}")]
    KeypairRead(String),
    #[error("pool api error")]
    PoolApi(#[from] ore_pool_api::error::ApiError),
    #[error("solana client error")]
    SolanaClient(#[from] solana_client::client_error::ClientError),
    #[error("invalid command")]
    InvalidCommand,
}
