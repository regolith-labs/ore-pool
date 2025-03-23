#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid helius cluster")]
    InvalidHeliusCluster,
    #[error("missing async solana client")]
    MissingHeliusSolanaAsyncClient,
    #[error("clock still ticking")]
    ClockStillTicking,
    #[error("unconfirmed jito bundle")]
    UnconfirmedJitoBundle,
    #[error("too many transactions in jito bundle")]
    TooManyTransactionsInJitoBundle,
    #[error("empty jito bundle")]
    EmptyJitoBundle,
    #[error("empty jito bundle confirmation")]
    EmptyJitoBundleConfirmation,
    #[error("empty tip accounts")]
    EmptyTipAccounts,
}
