use actix_web::HttpResponse;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("bincode")]
    Bincode(#[from] bincode::Error),
    #[error("base64 decode")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("try from slice")]
    TryFromSlice(#[from] std::array::TryFromSliceError),
    #[error("tokio postgres")]
    TokioPostgres(#[from] tokio_postgres::Error),
    #[error("deadpool postgress error")]
    DeadpoolPostgres(#[from] deadpool_postgres::PoolError),
    #[error("std io")]
    StdIO(#[from] std::io::Error),
    #[error("std env")]
    StdEnv(#[from] std::env::VarError),
    #[error("std parse int")]
    StdParseInt(#[from] std::num::ParseIntError),
    #[error("solana client")]
    SolanaClient(#[from] solana_client::client_error::ClientError),
    #[error("solana program")]
    SolanaProgram(#[from] solana_sdk::program_error::ProgramError),
    #[error("solana pubkey")]
    SolanaPubkey(#[from] solana_sdk::pubkey::ParsePubkeyError),
    #[error("member already exists")]
    MemberAlreadyExisits,
    #[error("member doesn't exist yet")]
    MemberDoesNotExist,
    #[error("{0}")]
    Internal(String),
}

impl From<Error> for HttpResponse {
    fn from(value: Error) -> Self {
        match value {
            Error::MemberDoesNotExist => HttpResponse::NotFound().finish(),
            _ => HttpResponse::InternalServerError().finish(),
        }
    }
}
