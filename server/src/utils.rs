use std::collections::HashMap;

use actix_cors::Cors;
use actix_web::http::header;
use drillx::Solution;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};

/// The current challenge the pool is accepting solutions for.
pub type Challenge = [u8; 32];

/// Aggregates contributions from the pool members.
#[derive(Default)]
pub struct Aggregator {
    /// The set of contributions aggregated for the current challenge.
    pub contributions: HashMap<Pubkey, Contribution>,

    /// The total difficulty score of all the contributions aggregated so far.
    pub total_score: u64,
}

/// A recorded contribution from a particular member of the pool.
pub struct Contribution {
    /// The member who submitted this solution.
    pub member: Pubkey,

    /// The difficulty score of the solution.
    pub score: u64,

    /// The drillx solution submitted representing the member's best hash.
    pub solution: Solution,
}

pub fn create_cors() -> Cors {
    Cors::default()
        .allowed_origin_fn(|_origin, _req_head| {
            // origin.as_bytes().ends_with(b"ore.supply") || // Production origin
            // origin == "http://localhost:8080" // Local development origin
            true
        })
        .allowed_methods(vec!["GET", "POST"]) // Methods you want to allow
        .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
        .allowed_header(header::CONTENT_TYPE)
        .max_age(3600)
}

// const RPC_URL: &str = "https://devnet.helius-rpc.com/?api-key=1de92644-323b-4900-9041-13c02730955c";
const RPC_URL: &str =
    "https://mainnet.helius-rpc.com/?api-key=1de92644-323b-4900-9041-13c02730955c";
pub fn rpc_client() -> RpcClient {
    RpcClient::new_with_commitment(RPC_URL.to_string(), CommitmentConfig::confirmed())
}
