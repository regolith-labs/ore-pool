use std::{collections::HashMap, pin::Pin, str::FromStr, sync::Arc, vec};

use futures::{Future, StreamExt, TryFutureExt, TryStreamExt};
use ore_api::state::{Config, Proof};
use ore_pool_api::state::{Member, Pool, Share};
use ore_pool_types::Staker;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    account::Account,
    clock::Clock,
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Keypair,
    signer::{EncodableKey, Signer},
    sysvar,
};
use steel::AccountDeserialize;

use crate::{database, error::Error, tx};

pub const BUFFER_OPERATOR: u64 = 5;
const MIN_DIFFICULTY: Option<u64> = None;

pub struct Operator {
    /// The pool authority keypair.
    pub keypair: Keypair,

    /// Solana RPC client.
    pub rpc_client: RpcClient,

    /// Postgres connection pool.
    pub db_client: deadpool_postgres::Pool,

    /// The boost accounts for mining multipliers.
    pub boost_accounts: Vec<BoostAccount>,

    /// The operator commission in % percentage.
    /// Applied to the miner and staker rewards.
    pub operator_commission: u64,

    /// The staker commission in % percentage.
    /// The rest is given to miners to incentize participation.
    pub staker_commission: u64,
}

pub struct BoostAccount {
    /// The mint account used to derive the boost account.
    pub mint: Pubkey,

    /// The boost account.
    pub boost: Pubkey,

    /// The pool's stake account derived from the boost.
    pub stake: Pubkey,
}

/// Pool's structure to make locked multipliers.
/// TODO: get a better name
#[derive(Debug, Clone)]
pub struct LockedMultipliers {
    schedule_multiplier: HashMap<Pubkey, Vec<(u64, u64)>>,
}

impl LockedMultipliers {
    pub fn from_map(schedule_multiplier: HashMap<Pubkey, Vec<(u64, u64)>>) -> Self {
        Self {
            schedule_multiplier,
        }
    }

    pub fn from_path(locked_multiplier_file_path: &str) -> Result<Self, Error> {
        let multipliers = crate::utils::load_locked_multipliers(locked_multiplier_file_path)?;
        Ok(Self {
            schedule_multiplier: multipliers.schedule_multiplier,
        })
    }

    pub fn calculate_lock_multiplier(&self, boost: &Pubkey, last_withdrawal: u64) -> u128 {
        self.schedule_multiplier
            .get(&boost)
            .unwrap_or(&vec![])
            .iter()
            .fold(1, |acc, (time, multiplier)| {
                if last_withdrawal >= *time {
                    acc * multiplier
                } else {
                    acc
                }
            }) as _
    }
}

impl BoostAccount {
    fn new(mint: Pubkey, operator_pubkey: Pubkey) -> Self {
        let (boost, _) = ore_boost_api::state::boost_pda(mint);
        let (pool, _) = ore_pool_api::state::pool_pda(operator_pubkey);
        let (stake, _) = ore_boost_api::state::stake_pda(pool, boost);
        Self { mint, boost, stake }
    }

    fn new_from_vec(mint_vec: Vec<Pubkey>, operator_pubkey: Pubkey) -> Vec<Self> {
        mint_vec
            .into_iter()
            .map(|mint| Self::new(mint, operator_pubkey))
            .collect()
    }
}

impl Operator {
    pub fn new() -> Result<Operator, Error> {
        let keypair = Self::keypair()?;
        let rpc_client = Self::rpc_client()?;
        let db_client = database::create_pool();
        let boosts = Self::load_boosts()?;
        log::info!("boosts: {:?}", boosts);
        let boost_accounts = BoostAccount::new_from_vec(boosts, keypair.pubkey());
        let operator_commission = Self::operator_commission()?;
        log::info!("operator commision: {}", operator_commission);
        let staker_commission = Self::staker_commission()?;
        log::info!("staker commission: {}", staker_commission);
        Ok(Operator {
            keypair,
            rpc_client,
            db_client,
            boost_accounts,
            operator_commission,
            staker_commission,
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

    pub async fn get_staker_onchain(
        &self,
        member_authority: &Pubkey,
        mint: &Pubkey,
    ) -> Result<(ore_pool_api::state::Share, Pubkey), Error> {
        let keypair = &self.keypair;
        let rpc_client = &self.rpc_client;
        let (pool_pda, _) = ore_pool_api::state::pool_pda(keypair.pubkey());
        let (share_pda, _) = ore_pool_api::state::share_pda(*member_authority, pool_pda, *mint);
        let data = rpc_client.get_account_data(&share_pda).await?;
        let share = ore_pool_api::state::Share::try_from_bytes(data.as_slice())?;
        Ok((*share, share_pda))
    }

    pub async fn get_staker_db(
        &self,
        member_authority: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Staker, Error> {
        let keypair = &self.keypair;
        let db_client = &self.db_client;
        let db_client = db_client.get().await?;
        let (pool_pda, _) = ore_pool_api::state::pool_pda(keypair.pubkey());
        let (share_pda, _) = ore_pool_api::state::share_pda(*member_authority, pool_pda, *mint);
        database::read_staker(&db_client, &share_pda.to_string()).await
    }

    pub async fn get_stakers_db(&self, mint: &Pubkey) -> Result<Vec<Pubkey>, Error> {
        let db_client = &self.db_client;
        let conn = db_client.get().await?;
        let stream = database::stream_stakers(&conn, mint)
            .await?
            .map(|staker| staker.map(|ok| ok.address));
        let vec: Vec<Pubkey> = stream.try_collect().await?;
        Ok(vec)
    }

    pub async fn get_stakers_db_as_string(&self, mint: &Pubkey) -> Result<Vec<String>, Error> {
        let db_client = &self.db_client;
        let conn = db_client.get().await?;
        let stream = database::stream_stakers(&conn, mint)
            .await?
            .map(|staker| staker.map(|ok| ok.address.to_string()));
        let vec: Vec<String> = stream.try_collect().await?;
        Ok(vec)
    }

    pub async fn get_stakers_onchain(
        &self,
        mint: &Pubkey,
    ) -> Result<HashMap<Pubkey, (u64, u64)>, Error> {
        let rpc_client = &self.rpc_client;
        let vec = self.get_stakers_db(mint).await?;
        let mut queries: Vec<Pin<Box<dyn Future<Output = GetManyStakers> + Send>>> = vec![];
        for chunk in vec.chunks(100) {
            let query = rpc_client
                .get_multiple_accounts(chunk)
                .map_err(Into::<Error>::into);
            queries.push(Box::pin(query));
        }
        let results: Vec<Vec<Option<Account>>> = futures::future::try_join_all(queries).await?;
        let results: HashMap<Pubkey, (u64, u64)> = results
            .into_iter()
            .flat_map(|v| v.into_iter())
            .filter_map(|option| {
                option.and_then(|account| {
                    let data = account.data;
                    let share = Share::try_from_bytes(data.as_slice()).ok();
                    share.map(|s| (s.authority, (s.balance, s.last_withdrawal)))
                })
            })
            .collect();
        Ok(results)
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

    pub async fn commit_stake(&self) -> Result<(), Error> {
        let authority = &self.keypair;
        let rpc_client = &self.rpc_client;
        let boost_mints = self.get_boosts();
        if boost_mints.len().gt(&0) {
            let mut ixs = vec![];
            for mint in boost_mints.iter() {
                let ix = ore_pool_api::sdk::commit(authority.pubkey(), *mint);
                ixs.push(ix);
            }
            let sig = tx::submit::submit_and_confirm_instructions(
                authority,
                rpc_client,
                ixs.as_slice(),
                1_000_000,
                10_000,
            )
            .await?;
            log::info!("commit stake sig: {:?}", sig);
        }
        Ok(())
    }

    /// the optional boost accounts for the mine instruction.
    pub fn get_boost_mine_accounts(&self) -> Vec<Pubkey> {
        let mut vec = vec![];
        let boost_accounts = &self.boost_accounts;
        for ba in boost_accounts.iter() {
            vec.push(ba.boost);
            vec.push(ba.stake);
        }
        vec
    }

    fn get_boosts(&self) -> Vec<Pubkey> {
        let boost_accounts = &self.boost_accounts;
        boost_accounts.iter().map(|ba| ba.mint).collect()
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

    fn load_boosts() -> Result<Vec<Pubkey>, Error> {
        let boost_1 = Self::boost_one()?;
        let boost_2 = Self::boost_two()?;
        let boost_3 = Self::boost_three()?;
        let boosts: Vec<Pubkey> = vec![boost_1, boost_2, boost_3]
            .into_iter()
            .flatten()
            .collect();
        Ok(boosts)
    }

    fn load_boost(var: String) -> Result<Option<Pubkey>, Error> {
        match std::env::var(var) {
            Ok(boost) => {
                let boost = Pubkey::from_str(boost.as_str())?;
                Ok(Some(boost))
            }
            // optional
            Err(_) => Ok(None),
        }
    }

    fn boost_one() -> Result<Option<Pubkey>, Error> {
        Self::load_boost("BOOST_ONE".to_string())
    }

    fn boost_two() -> Result<Option<Pubkey>, Error> {
        Self::load_boost("BOOST_TWO".to_string())
    }

    fn boost_three() -> Result<Option<Pubkey>, Error> {
        Self::load_boost("BOOST_THREE".to_string())
    }

    fn operator_commission() -> Result<u64, Error> {
        let str = std::env::var("OPERATOR_COMMISSION")?;
        let commission: u64 = str.parse()?;
        Ok(commission)
    }

    fn staker_commission() -> Result<u64, Error> {
        let str = std::env::var("STAKER_COMMISSION")?;
        let commission: u64 = str.parse()?;
        Ok(commission)
    }
}

type GetManyStakers = Result<Vec<Option<Account>>, Error>;

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
