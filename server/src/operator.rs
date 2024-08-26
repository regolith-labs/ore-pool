use ore_api::state::{Config, Proof};
use ore_utils::AccountDeserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    clock::Clock,
    commitment_config::CommitmentConfig,
    signature::Keypair,
    signer::{EncodableKey, Signer},
    sysvar,
};

use crate::error::Error;

const BUFFER_TIME: u64 = 5;

pub struct Operator {
    // The pool authority keypair.
    pub keypair: Keypair,

    // Solana RPC client.
    pub rpc_client: RpcClient,
}

impl Operator {
    pub fn new() -> Result<Operator, Error> {
        let keypair = Operator::keypair()?;
        let rpc_client = Operator::rpc_client()?;
        Ok(Operator {
            keypair,
            rpc_client,
        })
    }

    pub async fn get_proof(&self) -> Result<Proof, Error> {
        let authority = self.keypair.pubkey();
        let rpc_client = &self.rpc_client;
        let (pool_pda, _) = ore_pool_api::state::pool_pda(authority);
        let (proof_pda, _) = ore_pool_api::state::pool_proof_pda(pool_pda);
        let data = rpc_client.get_account_data(&proof_pda).await?;
        let proof = Proof::try_from_bytes(data.as_slice())?;
        Ok(*proof)
    }

    pub async fn get_config(&self) -> Result<Config, Error> {
        let config_pda = ore_api::consts::CONFIG_ADDRESS;
        let rpc_client = &self.rpc_client;
        let data = rpc_client.get_account_data(&config_pda).await?;
        let config = Config::try_from_bytes(data.as_slice())?;
        Ok(*config)
    }

    pub async fn get_cutoff(&self, proof: &Proof) -> Result<u64, Error> {
        let clock = self.get_clock().await?;
        Ok(proof
            .last_hash_at
            .saturating_add(60)
            .saturating_sub(BUFFER_TIME as i64)
            .saturating_sub(clock.unix_timestamp)
            .max(0) as u64)
    }

    async fn get_clock(&self) -> Result<Clock, Error> {
        let rpc_client = &self.rpc_client;
        let data = rpc_client.get_account_data(&sysvar::clock::id()).await?;
        bincode::deserialize(&data).map_err(From::from)
    }

    fn keypair() -> Result<Keypair, Error> {
        let keypair_path = Operator::keypair_path()?;
        let keypair = Keypair::read_from_file(keypair_path)
            .map_err(|err| Error::Internal(err.to_string()))?;
        Ok(keypair)
    }

    fn keypair_path() -> Result<String, Error> {
        std::env::var("KEYPAIR_PATH").map_err(From::from)
    }

    fn rpc_client() -> Result<RpcClient, Error> {
        let rpc_url = Operator::rpc_url()?;
        Ok(RpcClient::new_with_commitment(
            rpc_url,
            CommitmentConfig::confirmed(),
        ))
    }

    // "https://mainnet.helius-rpc.com/?api-key=1de92644-323b-4900-9041-13c02730955c";
    // const RPC_URL: &str = "https://devnet.helius-rpc.com/?api-key=1de92644-323b-4900-9041-13c02730955c";
    fn rpc_url() -> Result<String, Error> {
        std::env::var("RPC_URL").map_err(From::from)
    }
}
