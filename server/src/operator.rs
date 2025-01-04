use std::{str::FromStr, sync::Arc};

use ore_api::state::{Config, Proof};
use ore_pool_api::state::{Member, Pool};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    clock::Clock,
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Keypair,
    signer::{EncodableKey, Signer},
    sysvar,
};
use steel::AccountDeserialize;

use crate::{database, error::Error};

pub const BUFFER_OPERATOR: u64 = 5;
const MIN_DIFFICULTY: Option<u64> = Some(7);

pub struct Operator {
    /// The pool authority keypair.
    pub keypair: Keypair,

    /// Solana RPC client.
    pub rpc_client: RpcClient,

    /// Postgres connection pool.
    pub db_client: deadpool_postgres::Pool,

    /// The operator commission in % percentage.
    /// Applied to the miner and staker rewards.
    pub operator_commission: u64,
}

impl Operator {
    pub fn new() -> Result<Operator, Error> {
        let keypair = Self::keypair()?;
        let rpc_client = Self::rpc_client()?;
        let db_client = database::create_pool();
        let operator_commission = Self::operator_commission()?;
        log::info!("operator commision: {}", operator_commission);
        Ok(Operator {
            keypair,
            rpc_client,
            db_client,
            operator_commission,
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

    pub async fn get_member_onchain(&self, member_authority: &Pubkey) -> Result<Member, Error> {
        let authority = self.keypair.pubkey();
        let rpc_client = &self.rpc_client;
        let (pool_pda, _) = ore_pool_api::state::pool_pda(authority);
        let (member_pda, _) = ore_pool_api::state::member_pda(*member_authority, pool_pda);
        let data = rpc_client.get_account_data(&member_pda).await?;
        let member = Member::try_from_bytes(data.as_slice())?;
        Ok(*member)
    }

    pub async fn get_member_db(
        &self,
        member_authority: &str,
    ) -> Result<ore_pool_types::Member, Error> {
        let db_client = self.db_client.get().await?;
        let member_authority = Pubkey::from_str(member_authority)?;
        let pool_authority = self.keypair.pubkey();
        let (pool_pda, _) = ore_pool_api::state::pool_pda(pool_authority);
        let (member_pda, _) = ore_pool_api::state::member_pda(member_authority, pool_pda);
        database::read_member(&db_client, &member_pda.to_string()).await
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

    pub async fn attribute_members(self: Arc<Self>) -> Result<(), Error> {
        let db_client = self.db_client.get().await?;
        let db_client = Arc::new(db_client);
        database::stream_members_attribution(db_client, self).await?;
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

    fn operator_commission() -> Result<u64, Error> {
        let str = std::env::var("OPERATOR_COMMISSION")?;
        let commission: u64 = str.parse()?;
        Ok(commission)
    }
}
