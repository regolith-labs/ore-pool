use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    u128,
};

use drillx::Solution;
use ore_api::{
    consts::{BUS_ADDRESSES, BUS_COUNT},
    state::Bus,
};
use ore_pool_types::Challenge;
use rand::Rng;
use sha3::{Digest, Sha3_256};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use steel::AccountDeserialize;

use crate::{
    database,
    error::Error,
    operator::{Operator, BUFFER_OPERATOR},
    tx, webhook,
};

/// The client submits slightly earlier
/// than the operator's cutoff time to create a "submission window".
pub const BUFFER_CLIENT: u64 = 2 + BUFFER_OPERATOR;
const MAX_DIFFICULTY: u32 = 22;
const MAX_SCORE: u64 = 2u64.pow(MAX_DIFFICULTY);

/// Aggregates contributions from the pool members.
pub struct Aggregator {
    /// The current challenge.
    pub challenge: Challenge,

    /// The set of contributions for attribution.
    pub contributions: Miners,

    /// The total difficulty score of all the contributions aggregated so far.
    pub total_score: u64,

    /// The best solution submitted.
    pub winner: Option<Winner>,

    /// The number of workers that have been approved for the current challenge.
    pub num_members: u64,

    /// The map of stake contributors for attribution.
    pub stake: Stakers,
}

/// Miners
pub type LastHashAt = u64;
pub type MinerContributions = HashSet<Contribution>;
pub type Miners = HashMap<LastHashAt, MinerContributions>;

/// Stakers
pub type BoostMint = Pubkey;
pub type StakerBalances = HashMap<Pubkey, u64>;
pub type Stakers = HashMap<BoostMint, StakerBalances>;

// Best hash to be submitted for the current challenge.
#[derive(Clone, Copy, Debug)]
pub struct Winner {
    // The winning solution.
    pub solution: Solution,

    // The current largest difficulty.
    pub difficulty: u32,
}

/// A recorded contribution from a particular member of the pool.
#[derive(Clone, Copy, Debug)]
pub struct Contribution {
    /// The member who submitted this solution.
    pub member: Pubkey,

    /// The difficulty score of the solution.
    pub score: u64,

    /// The drillx solution submitted representing the member's best hash.
    pub solution: Solution,
}

impl PartialEq for Contribution {
    fn eq(&self, other: &Self) -> bool {
        self.member == other.member
    }
}

impl Eq for Contribution {}

impl Hash for Contribution {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.member.hash(state);
    }
}

pub async fn process_contributions(
    aggregator: &tokio::sync::RwLock<Aggregator>,
    operator: &Operator,
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<Contribution>,
) -> Result<(), Error> {
    // outer loop for new challenges
    loop {
        let timer = tokio::time::Instant::now();
        let cutoff_time = {
            let proof = match operator.get_proof().await {
                Ok(proof) => proof,
                Err(err) => {
                    log::error!("{:?}", err);
                    continue;
                }
            };
            match operator.get_cutoff(&proof).await {
                Ok(cutoff_time) => cutoff_time,
                Err(err) => {
                    log::error!("{:?}", err);
                    continue;
                }
            }
        };
        let mut remaining_time = cutoff_time.saturating_sub(timer.elapsed().as_secs());
        // inner loop to process contributions until cutoff time
        while remaining_time > 0 {
            // race the next contribution against remaining time
            match tokio::time::timeout(tokio::time::Duration::from_secs(remaining_time), rx.recv())
                .await
            {
                Ok(Some(mut contribution)) => {
                    {
                        let mut aggregator = aggregator.write().await;
                        let _ = aggregator.insert(&mut contribution);
                    }
                    // recalculate the remaining time after processing the contribution
                    remaining_time = cutoff_time.saturating_sub(timer.elapsed().as_secs());
                }
                Ok(None) => {
                    // if the receiver is closed, exit server
                    return Err(Error::Internal("contribution channel closed".to_string()));
                }
                Err(_) => {
                    // timeout expired, meaning cutoff time has been reached
                    break;
                }
            }
        }
        // at this point, the cutoff time has been reached
        let total_score = {
            let read = aggregator.read().await;
            read.total_score
        };
        if total_score > 0 {
            // submit if contributions exist
            let mut aggregator = aggregator.write().await;
            if let Err(err) = aggregator.submit_and_reset(operator).await {
                log::error!("{:?}", err);
            }
        } else {
            // no contributions yet, wait for the first one to submit
            if let Some(mut contribution) = rx.recv().await {
                let mut aggregator = aggregator.write().await;
                let _ = aggregator.insert(&mut contribution);
                if let Err(err) = aggregator.submit_and_reset(operator).await {
                    log::error!("{:?}", err);
                }
            }
        }
    }
}

impl Aggregator {
    pub async fn new(operator: &Operator) -> Result<Self, Error> {
        // fetch accounts
        let pool = operator.get_pool().await?;
        let proof = operator.get_proof().await?;
        log::info!("proof: {:?}", proof);
        let cutoff_time = operator.get_cutoff(&proof).await?;
        let min_difficulty = operator.min_difficulty().await?;
        let challenge = Challenge {
            challenge: proof.challenge,
            lash_hash_at: proof.last_hash_at,
            min_difficulty,
            cutoff_time,
        };
        // fetch staker balances
        let mut stake: Stakers = HashMap::new();
        let boost_acounts = operator.boost_accounts.iter();
        for ba in boost_acounts {
            let stakers = operator.get_stakers_onchain(&ba.mint).await?;
            stake.insert(ba.mint, stakers);
        }
        // build self
        let mut contributions = HashMap::new();
        contributions.insert(challenge.lash_hash_at as u64, HashSet::new());
        let aggregator = Aggregator {
            challenge,
            contributions,
            total_score: 0,
            winner: None,
            num_members: pool.last_total_members,
            stake,
        };
        Ok(aggregator)
    }

    fn insert(&mut self, contribution: &mut Contribution) -> Result<(), Error> {
        // normalize contribution score
        let normalized_score = contribution.score.min(MAX_SCORE);
        contribution.score = normalized_score;
        // get current contributions
        let contributions = self.get_current_contributions()?;
        // insert
        let insert = contributions.insert(*contribution);
        match insert {
            true => {
                let difficulty = contribution.solution.to_hash().difficulty();
                let contender = Winner {
                    solution: contribution.solution,
                    difficulty,
                };
                self.total_score += contribution.score;
                match self.winner {
                    Some(winner) => {
                        if difficulty > winner.difficulty {
                            self.winner = Some(contender);
                        }
                    }
                    None => self.winner = Some(contender),
                }
                Ok(())
            }
            false => {
                log::error!("already received contribution: {:?}", contribution.member);
                Ok(())
            }
        }
    }

    // TODO Publish block to S3
    async fn submit_and_reset(&mut self, operator: &Operator) -> Result<(), Error> {
        // check if reset is needed
        // this may happen if a solution is landed on chain
        // but a subsequent application error is thrown before resetting
        if self.check_for_reset(operator).await? {
            self.reset(operator).await?;
            // there was a reset
            // so restart contribution loop against new challenge
            return Ok(());
        };
        // prepare best solution and attestation of hash-power
        let winner = self.winner()?;
        log::info!("winner: {:?}", winner);
        let best_solution = winner.solution;
        let attestation = self.attestation()?;
        // derive accounts for instructions
        let authority = &operator.keypair.pubkey();
        let (pool_pda, _) = ore_pool_api::state::pool_pda(*authority);
        let (pool_proof_pda, _) = ore_pool_api::state::pool_proof_pda(pool_pda);
        let bus = self.find_bus(operator).await?;
        // build instructions
        let auth_ix = ore_api::sdk::auth(pool_proof_pda);
        let submit_ix = ore_pool_api::sdk::submit(
            operator.keypair.pubkey(),
            best_solution,
            attestation,
            bus,
            operator.get_boost_mine_accounts(),
        );
        let rpc_client = &operator.rpc_client;
        let sig = tx::submit::submit_and_confirm_instructions(
            &operator.keypair,
            rpc_client,
            &[auth_ix, submit_ix],
            1_500_000,
            500_000,
        )
        .await?;
        log::info!("{:?}", sig);
        // reset
        self.reset(operator).await?;
        Ok(())
    }

    pub async fn distribute_rewards(
        &mut self,
        operator: &Operator,
        rewards: &(ore_api::event::MineEvent, webhook::BoostAccounts),
    ) -> Result<(), Error> {
        let (rewards, boost_acounts) = rewards;
        let (pool_pda, _) = ore_pool_api::state::pool_pda(operator.keypair.pubkey());
        // compute attributions for miners
        log::info!("reward: {:?}", rewards);
        log::info!("// miner ////////////////////////");
        let rewards_distribution = self.rewards_distribution(
            pool_pda,
            rewards,
            operator.operator_commission,
            operator.staker_commission,
        )?;
        log::info!("// staker ////////////////////////");
        // compute attributions for stakers
        let rewards_distribution_boost_1 = self
            .rewards_distribution_boost(
                operator,
                pool_pda,
                boost_acounts.one.map(|p| (rewards.boost_1, p)),
                operator.staker_commission,
            )
            .await?;
        let rewards_distribution_boost_2 = self
            .rewards_distribution_boost(
                operator,
                pool_pda,
                boost_acounts.two.map(|p| (rewards.boost_2, p)),
                operator.staker_commission,
            )
            .await?;
        let rewards_distribution_boost_3 = self
            .rewards_distribution_boost(
                operator,
                pool_pda,
                boost_acounts.three.map(|p| (rewards.boost_3, p)),
                operator.staker_commission,
            )
            .await?;
        log::info!("// operator ////////////////////////");
        // compute attribution for operator
        let rewards_distribution_operator = self.rewards_distribution_operator(
            pool_pda,
            operator.keypair.pubkey(),
            rewards,
            operator.operator_commission,
        );
        // write rewards to db
        let mut db_client = operator.db_client.get().await?;
        database::write_member_total_balances(&mut db_client, rewards_distribution).await?;
        database::write_member_total_balances(&mut db_client, rewards_distribution_boost_1).await?;
        database::write_member_total_balances(&mut db_client, rewards_distribution_boost_2).await?;
        database::write_member_total_balances(&mut db_client, rewards_distribution_boost_3).await?;
        database::write_member_total_balances(&mut db_client, vec![rewards_distribution_operator])
            .await?;
        // clean up contributions
        let contributions = &mut self.contributions;
        let _ = contributions.remove(&(rewards.last_hash_at as u64));
        Ok(())
    }

    fn rewards_distribution(
        &self,
        pool: Pubkey,
        rewards: &ore_api::event::MineEvent,
        operator_commission: u64,
        staker_commission: u64,
    ) -> Result<Vec<(String, u64)>, Error> {
        let contributions = &self.contributions;
        let contributions =
            contributions
                .get(&(rewards.last_hash_at as u64))
                .ok_or(Error::Internal(
                    "missing contributions at reward hash".to_string(),
                ))?;
        // compute denominator
        let denominator: u64 = contributions.iter().map(|c| c.score).sum();
        let denominator: u128 = denominator as u128;
        // compute base mine rewards
        let mine_rewards = rewards.reward - rewards.boost_1 - rewards.boost_2 - rewards.boost_3;
        log::info!("base reward denominator: {}", denominator);
        // compute miner split
        let miner_commission = 100 - operator_commission;
        log::info!("miner commission: {}", miner_commission);
        let miner_rewards = (mine_rewards as u128)
            .saturating_mul(miner_commission as u128)
            .saturating_div(100);
        log::info!("miner rewards as commission for miners: {}", miner_rewards);
        // compute miner split from stake rewards
        let stake_rewards = rewards.boost_1 + rewards.boost_2 + rewards.boost_3;
        let miner_rewards_from_stake = Self::split_stake_rewards_for_miners(
            stake_rewards,
            operator_commission,
            staker_commission,
        );
        let total_rewards = miner_rewards + miner_rewards_from_stake;
        log::info!("total rewards as commission for miners: {}", total_rewards);
        let contributions = contributions.iter();
        let distribution = contributions
            .map(|c| {
                log::info!("raw base reward score: {}", c.score);
                let score = (c.score as u128).saturating_mul(total_rewards);
                let score = score.checked_div(denominator).unwrap_or(0);
                log::info!("attributed base reward score: {}", score);
                let (member_pda, _) = ore_pool_api::state::member_pda(c.member, pool);
                (member_pda.to_string(), score as u64)
            })
            .collect();
        Ok(distribution)
    }

    fn split_stake_rewards_for_miners(
        stake_rewards: u64,
        operator_commission: u64,
        staker_commission: u64,
    ) -> u128 {
        let miner_commission_for_stake: u128 =
            (100 - operator_commission - staker_commission) as u128;
        log::info!("miner commission for stake: {}", miner_commission_for_stake);
        let stake_rewards = stake_rewards as u128;
        let miner_rewards_from_stake = stake_rewards
            .saturating_mul(miner_commission_for_stake)
            .saturating_div(100);
        log::info!(
            "stake rewards as commission for miners: {}",
            miner_rewards_from_stake
        );
        miner_rewards_from_stake
    }

    async fn rewards_distribution_boost(
        &self,
        operator: &Operator,
        pool: Pubkey,
        boost_event: Option<(u64, Pubkey)>,
        staker_commission: u64,
    ) -> Result<Vec<(String, u64)>, Error> {
        match boost_event {
            None => Ok(vec![]),
            Some((boost_reward, boost_account)) => {
                let boost_data = operator.rpc_client.get_account_data(&boost_account).await?;
                let boost = ore_boost_api::state::Boost::try_from_bytes(boost_data.as_slice())?;
                let boost_mint = boost.mint;
                log::info!("{:?}", (boost_reward, boost_mint));
                let total_reward = boost_reward as u128;
                let staker_commission: u128 = staker_commission as u128;
                log::info!("staker commission: {}", staker_commission);
                let staker_rewards = total_reward
                    .saturating_mul(staker_commission)
                    .saturating_div(100);
                log::info!("total rewards from stake: {}", total_reward);
                log::info!(
                    "total rewards as commission for stakers: {}",
                    staker_rewards
                );
                let stakers = self.stake.get(&boost_mint).ok_or(Error::Internal(format!(
                    "missing staker balances: {:?}",
                    boost_mint,
                )))?;
                let denominator_iter = stakers.iter();
                let distribution_iter = stakers.iter();
                let denominator: u64 = denominator_iter.map(|(_, balance)| balance).sum();
                let denominator = denominator as u128;
                log::info!("staked reward denominator: {}", denominator);
                let res = distribution_iter
                    .map(|(stake_authority, balance)| {
                        log::info!("staked balance: {:?}", (stake_authority, balance));
                        let balance = *balance as u128;
                        let score = balance.saturating_mul(staker_rewards);
                        log::info!("scaled score from stake: {}", score);
                        let score = score.checked_div(denominator).unwrap_or(0);
                        log::info!("attributed reward from stake: {}", score);
                        let (member_pda, _) =
                            ore_pool_api::state::member_pda(*stake_authority, pool);
                        (member_pda.to_string(), score as u64)
                    })
                    .collect();
                Ok(res)
            }
        }
    }

    fn rewards_distribution_operator(
        &self,
        pool: Pubkey,
        pool_authority: Pubkey,
        rewards: &ore_api::event::MineEvent,
        operator_commission: u64,
    ) -> (String, u64) {
        log::info!("operator commission: {}", operator_commission);
        let total_rewards = (rewards.reward as u128)
            .saturating_mul(operator_commission as u128)
            .saturating_div(100);
        let total_rewards = total_rewards as u64;
        log::info!("total rewards for operator: {}", total_rewards);
        let (member_pda, _) = ore_pool_api::state::member_pda(pool_authority, pool);
        (member_pda.to_string(), total_rewards)
    }

    async fn find_bus(&self, operator: &Operator) -> Result<Pubkey, Error> {
        // Fetch the bus with the largest balance
        let rpc_client = &operator.rpc_client;
        let accounts = rpc_client.get_multiple_accounts(&BUS_ADDRESSES).await?;
        let mut top_bus_balance: u64 = 0;
        let bus_index = rand::thread_rng().gen_range(0..BUS_COUNT);
        let mut top_bus = BUS_ADDRESSES[bus_index];
        for account in accounts.into_iter().flatten() {
            if let Ok(bus) = Bus::try_from_bytes(&account.data) {
                if bus.rewards.gt(&top_bus_balance) {
                    top_bus_balance = bus.rewards;
                    top_bus = BUS_ADDRESSES[bus.id as usize];
                }
            }
        }
        Ok(top_bus)
    }

    fn attestation(&mut self) -> Result<[u8; 32], Error> {
        let mut hasher = Sha3_256::new();
        let contributions = self.get_current_contributions()?;
        let num_contributions = contributions.len();
        log::info!("num contributions: {}", num_contributions);
        for contribution in contributions.iter() {
            let hex_string: String =
                contribution
                    .solution
                    .d
                    .iter()
                    .fold(String::new(), |mut acc, byte| {
                        acc.push_str(&format!("{:02x}", byte));
                        acc
                    });
            let line = format!(
                "{} {} {}\n",
                contribution.member,
                hex_string,
                u64::from_le_bytes(contribution.solution.n)
            );
            hasher.update(&line);
        }
        let mut attestation: [u8; 32] = [0; 32];
        attestation.copy_from_slice(&hasher.finalize()[..]);
        Ok(attestation)
    }

    fn get_current_contributions(&mut self) -> Result<&mut MinerContributions, Error> {
        let last_hash_at = self.challenge.lash_hash_at as u64;
        let contributions = &mut self.contributions;
        let contributions = contributions.get_mut(&last_hash_at).ok_or(Error::Internal(
            "missing contributions at current hash".to_string(),
        ))?;
        Ok(contributions)
    }

    async fn reset(&mut self, operator: &Operator) -> Result<(), Error> {
        log::info!("//////////////////////////////////////////");
        log::info!("resetting");
        log::info!("//////////////////////////////////////////");
        // update challenge
        self.update_challenge(operator).await?;
        // allocate key for new contributions
        let last_hash_at = self.challenge.lash_hash_at as u64;
        let contributions = &mut self.contributions;
        log::info!("//////////////////////////////////////////");
        log::info!("new contributions key: {:?}", last_hash_at);
        log::info!("//////////////////////////////////////////");
        if let Some(_) = contributions.insert(last_hash_at, HashSet::new()) {
            log::error!("contributions at last-hash-at already exist");
        }
        // reset accumulators
        let pool = operator.get_pool().await?;
        self.total_score = 0;
        self.winner = None;
        self.num_members = pool.last_total_members;
        Ok(())
    }

    fn winner(&self) -> Result<Winner, Error> {
        self.winner
            .ok_or(Error::Internal("no solutions were submitted".to_string()))
    }

    async fn check_for_reset(&self, operator: &Operator) -> Result<bool, Error> {
        let last_hash_at = self.challenge.lash_hash_at;
        let proof = operator.get_proof().await?;
        let needs_reset = proof.last_hash_at != last_hash_at;
        Ok(needs_reset)
    }

    async fn update_challenge(&mut self, operator: &Operator) -> Result<(), Error> {
        let max_retries = 10;
        let mut retries = 0;
        let last_hash_at = self.challenge.lash_hash_at;
        loop {
            let proof = operator.get_proof().await?;
            if proof.last_hash_at != last_hash_at {
                let cutoff_time = operator.get_cutoff(&proof).await?;
                let min_difficulty = operator.min_difficulty().await?;
                self.challenge.challenge = proof.challenge;
                self.challenge.lash_hash_at = proof.last_hash_at;
                self.challenge.min_difficulty = min_difficulty;
                self.challenge.cutoff_time = cutoff_time;
                return Ok(());
            } else {
                retries += 1;
                if retries == max_retries {
                    return Err(Error::Internal("failed to fetch new challenge".to_string()));
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}
