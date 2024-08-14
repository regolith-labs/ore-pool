use actix_cors::Cors;
use actix_web::http::header;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;

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
