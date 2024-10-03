use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use drillx::Solution;
use ore_api::{
    consts::{BUS_ADDRESSES, BUS_COUNT},
    state::Bus,
};
use ore_utils::AccountDeserialize;
use rand::Rng;
use sha3::{Digest, Sha3_256};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use types::Challenge;

use crate::{
    database,
    error::Error,
    operator::{Operator, BUFFER_OPERATOR},
    tx,
};

/// The client submits slightly earlier
/// than the operator's cutoff time to create a "submission window".
pub const BUFFER_CLIENT: u64 = 2 + BUFFER_OPERATOR;

/// Aggregates contributions from the pool members.
pub struct Aggregator {
    /// The current challenge.
    pub challenge: Challenge,

    /// The set of contributions aggregated for the current challenge.
    pub contributions: HashSet<Contribution>,

    /// The total difficulty score of all the contributions aggregated so far.
    pub total_score: u64,

    /// The best solution submitted.
    pub winner: Option<Winner>,

    /// The number of workers that have been approved for the current challenge.
    pub num_members: u64,

    /// The map of stake contributors for attribution.
    pub stake: HashMap<Pubkey, u64>,
}

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
            let read = aggregator.read().await;
            read.challenge.cutoff_time
        };
        let mut remaining_time = cutoff_time.saturating_sub(timer.elapsed().as_secs());
        // inner loop to process contributions until cutoff time
        while remaining_time > 0 {
            // race the next contribution against remaining time
            match tokio::time::timeout(tokio::time::Duration::from_secs(remaining_time), rx.recv())
                .await
            {
                Ok(Some(contribution)) => {
                    {
                        let mut aggregator = aggregator.write().await;
                        aggregator.insert(&contribution);
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
            if let Some(contribution) = rx.recv().await {
                let mut aggregator = aggregator.write().await;
                aggregator.insert(&contribution);
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
            lash_hash_at: pool.last_hash_at,
            min_difficulty,
            cutoff_time,
        };
        // fetch boosts
        let boost_mint_1 = operator
            .boost_accounts
            .first()
            .ok_or(Error::Internal("missing boost account".to_string()))?;
        // fetch staker balances
        let stakers = operator.get_stakers_onchain(&boost_mint_1.mint).await?;
        // build self
        let aggregator = Aggregator {
            challenge,
            contributions: HashSet::new(),
            total_score: 0,
            winner: None,
            num_members: pool.last_total_members,
            stake: stakers,
        };
        Ok(aggregator)
    }

    fn insert(&mut self, contribution: &Contribution) {
        match self.contributions.insert(*contribution) {
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
            }
            false => {
                log::error!("already received contribution: {:?}", contribution.member);
            }
        }
    }

    // TODO Publish block to S3
    async fn submit_and_reset(&mut self, operator: &Operator) -> Result<(), Error> {
        // check if reset is needed
        // this may happen if a solution is landed on chain
        // but a subsequent application error is thrown before resetting
        if self.check_for_reset(operator).await? {
            log::error!("irregular reset");
            self.reset(operator).await?;
        };
        // prepare best solution and attestation of hash-power
        let winner = self.winner()?;
        log::info!("winner: {:?}", winner);
        let best_solution = winner.solution;
        let attestation = self.attestation();
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
        // parse mining reward
        let reward = self.parse_reward(operator).await?;
        log::info!("reward: {}", reward);
        let rewards_distribution = self.rewards_distribution(pool_pda, reward);
        // write rewards to db
        let mut db_client = operator.db_client.get().await?;
        database::write_member_total_balances(&mut db_client, rewards_distribution).await?;
        // reset
        self.reset(operator).await?;
        Ok(())
    }

    fn rewards_distribution(&self, pool: Pubkey, reward: u64) -> Vec<(String, u64)> {
        let denominator = self.total_score;
        log::info!("denominator: {}", denominator);
        let contributions = self.contributions.iter();
        contributions
            .map(|c| {
                log::info!("raw score: {}", c.score);
                let score = c.score.saturating_mul(reward);
                let score = score.checked_div(denominator).unwrap_or(0);
                log::info!("attributed score: {}", score);
                let (member_pda, _) = ore_pool_api::state::member_pda(c.member, pool);
                (member_pda.to_string(), score)
            })
            .collect()
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

    fn attestation(&self) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        let contributions = &self.contributions;
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
        attestation
    }

    async fn reset(&mut self, operator: &Operator) -> Result<(), Error> {
        self.update_challenge(operator).await?;
        let pool = operator.get_pool().await?;
        self.contributions = HashSet::new();
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
        let pool = operator.get_pool().await?;
        let needs_reset = pool.last_hash_at != last_hash_at;
        Ok(needs_reset)
    }

    async fn parse_reward(&self, operator: &Operator) -> Result<u64, Error> {
        let max_retries = 20;
        let mut retries = 0;
        let last_hash_at = self.challenge.lash_hash_at;
        loop {
            let pool = operator.get_pool().await?;
            if pool.last_hash_at != last_hash_at {
                return Ok(pool.reward);
            } else {
                retries += 1;
                if retries == max_retries {
                    return Err(Error::Internal("failed to fetch reward".to_string()));
                }
                log::info!("reward retries: {}", retries);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }

    async fn update_challenge(&mut self, operator: &Operator) -> Result<(), Error> {
        let max_retries = 10;
        let mut retries = 0;
        let last_hash_at = self.challenge.lash_hash_at;
        loop {
            let proof = operator.get_proof().await?;
            let pool = operator.get_pool().await?;
            if pool.last_hash_at != last_hash_at {
                let cutoff_time = operator.get_cutoff(&proof).await?;
                let min_difficulty = operator.min_difficulty().await?;
                self.challenge.challenge = proof.challenge;
                self.challenge.lash_hash_at = pool.last_hash_at;
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
