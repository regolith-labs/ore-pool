use base64::prelude::*;
use ore_api::state::{Config, Proof};
use ore_pool_api::state::{Member, Pool};
use ore_utils::AccountDeserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    clock::Clock,
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::{EncodableKey, Signer},
    sysvar,
};
use solana_transaction_status::{
    option_serializer::OptionSerializer, EncodedConfirmedTransactionWithStatusMeta,
    UiTransactionEncoding,
};

use crate::{database, error::Error};

pub const BUFFER_OPERATOR: u64 = 5;
const MIN_DIFFICULTY: Option<u64> = None;

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

    pub async fn get_pool(&self) -> Result<Pool, Error> {
        let authority = self.keypair.pubkey();
        let rpc_client = &self.rpc_client;
        let (pool_pda, _) = ore_pool_api::state::pool_pda(authority);
        let data = rpc_client.get_account_data(&pool_pda).await?;
        let pool = Pool::try_from_bytes(data.as_slice())?;
        Ok(*pool)
    }

    pub async fn get_member(&self, member_authority: &Pubkey) -> Result<Member, Error> {
        let authority = self.keypair.pubkey();
        let rpc_client = &self.rpc_client;
        let (pool_pda, _) = ore_pool_api::state::pool_pda(authority);
        let (member_pda, _) = ore_pool_api::state::member_pda(*member_authority, pool_pda);
        let data = rpc_client.get_account_data(&member_pda).await?;
        let member = Member::try_from_bytes(data.as_slice())?;
        Ok(*member)
    }

    pub async fn get_proof(&self) -> Result<Proof, Error> {
        let authority = self.keypair.pubkey();
        let rpc_client = &self.rpc_client;
        let (pool_pda, _) = ore_pool_api::state::pool_pda(authority);
        log::info!("pool pda: {}", pool_pda);
        let (proof_pda, _) = ore_pool_api::state::pool_proof_pda(pool_pda);
        log::info!("proof pda: {}", proof_pda);
        let data = rpc_client.get_account_data(&proof_pda).await?;
        let proof = Proof::try_from_bytes(data.as_slice())?;
        Ok(*proof)
    }

    pub async fn get_cutoff(&self, proof: &Proof) -> Result<u64, Error> {
        let clock = self.get_clock().await?;
        Ok(proof
            .last_hash_at
            .saturating_add(60)
            .saturating_sub(BUFFER_OPERATOR as i64)
            .saturating_sub(clock.unix_timestamp)
            .max(0) as u64)
    }

    pub async fn min_difficulty(&self) -> Result<u64, Error> {
        let config = self.get_config().await?;
        let program_min = config.min_difficulty;
        match MIN_DIFFICULTY {
            Some(operator_min) => {
                let max = program_min.max(operator_min);
                Ok(max)
            }
            None => Ok(program_min),
        }
    }

    // TODO: try writing this data to the pool account
    pub async fn parse_reward_with_retries(&self, sig: &Signature) -> Result<u64, Error> {
        let mut retries = 0;
        let max_retries = 10;
        while retries < max_retries {
            match self.parse_reward(sig).await {
                reward @ Ok(_) => {
                    return reward;
                }
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    retries += 1;
                    log::info!("retries: {}", retries);
                }
            }
        }
        Err(Error::Internal(
            "could not get transaction from rpc".to_string(),
        ))
    }

    async fn parse_reward(&self, sig: &Signature) -> Result<u64, Error> {
        let rpc_client = &self.rpc_client;
        let tx = rpc_client
            .get_transaction(sig, UiTransactionEncoding::Json)
            .await?;
        if let Some(meta) = tx.transaction.meta {
            if let OptionSerializer::Some(return_data) = meta.return_data {
                let (data, _) = return_data.data;
                let bytes = BASE64_STANDARD.decode(data)?;
                let event: &ore_api::event::MineEvent = bytemuck::try_from_bytes(bytes.as_slice())
                    .map_err(|err| Error::Internal(err.to_string()))?;
                return Ok(event.reward);
            }
        }
        Err(Error::Internal("could not parse return data".to_string()))
    }

    pub async fn attribute_members(
        &self,
        db_client: &deadpool_postgres::Pool,
    ) -> Result<(), Error> {
        let db_client = db_client.get().await?;
        database::stream_members_attribution(&db_client, self).await?;
        Ok(())
    }

    async fn get_config(&self) -> Result<Config, Error> {
        let config_pda = ore_api::consts::CONFIG_ADDRESS;
        let rpc_client = &self.rpc_client;
        let data = rpc_client.get_account_data(&config_pda).await?;
        let config = Config::try_from_bytes(data.as_slice())?;
        Ok(*config)
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

    fn rpc_url() -> Result<String, Error> {
        std::env::var("RPC_URL").map_err(From::from)
    }
}

#[cfg(test)]
mod tests {
    use base64::{prelude::BASE64_STANDARD, Engine};
    use ore_api::event::MineEvent;

    #[test]
    fn one() {
        let bytes = vec![
            10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 245, 15, 0, 0, 0, 0, 0, 0,
        ];
        let res = bytemuck::try_from_bytes::<MineEvent>(bytes.as_slice()).cloned();
        println!("res: {:?}", res);
    }

    #[test]
    fn two() {
        let base64 = "CgAAAAAAAAAAAAAAAAAAAPUPAAAAAAAA".to_string();
        let bytes = BASE64_STANDARD.decode(base64);
        println!("bytes: {:?}", bytes);
    }
}
