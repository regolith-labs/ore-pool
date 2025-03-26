use std::env::VarError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("std env")]
    StdEnv(#[from] VarError),
    #[error("std io")]
    StdIo(#[from] std::io::Error),
    #[error("could not ready keypair from provided path: {0}")]
    KeypairRead(String),
    #[error("pool api")]
    PoolApi(#[from] ore_pool_api::error::ApiError),
    #[error("solana client")]
    SolanaClient(#[from] solana_client::client_error::ClientError),
    #[error("solana program")]
    SolaanProgram(#[from] solana_program::program_error::ProgramError),
    #[error("solana parse pubkey")]
    SolanaParsePubkey(#[from] solana_sdk::pubkey::ParsePubkeyError),
    #[error("missing pool url")]
    MissingPoolUrl,
    #[error("invalid command")]
    InvalidCommand,
    #[error("member pool mismatch")]
    MemberPoolMismatch,
}
