use actix_web::{http::header::ToStrError, HttpResponse};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("bincode")]
    Bincode(#[from] bincode::Error),
    #[error("base64 decode")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("try from slice")]
    TryFromSlice(#[from] std::array::TryFromSliceError),
    #[error("rewards channel send")]
    RewardsChannelSend(#[from] tokio::sync::mpsc::error::SendError<ore_api::event::MineEvent>),
    #[error("tokio postgres")]
    TokioPostgres(#[from] tokio_postgres::Error),
    #[error("deadpool postgress")]
    DeadpoolPostgres(#[from] deadpool_postgres::PoolError),
    #[error("http header to string")]
    HttpHeader(#[from] ToStrError),
    #[error("reqwest")]
    Reqwest(#[from] reqwest::Error),
    #[error("serde json")]
    SerdeJson(#[from] serde_json::Error),
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
    #[error("member doesn't exist yet")]
    MemberDoesNotExist,
    #[error("staker doesn't exist yet")]
    StakerDoesNotExist,
    #[error("share account received")]
    ShareAccountReceived,
    #[error("proof account received")]
    ProofAccountReceived,
    #[error("{0}")]
    Internal(String),
}

impl From<Error> for HttpResponse {
    fn from(value: Error) -> Self {
        match value {
            Error::MemberDoesNotExist | Error::StakerDoesNotExist => {
                HttpResponse::NotFound().finish()
            }
            Error::ShareAccountReceived => HttpResponse::Ok().finish(),
            _ => HttpResponse::InternalServerError().finish(),
        }
    }
}
