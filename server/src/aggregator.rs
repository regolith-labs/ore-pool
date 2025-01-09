use std::collections::HashMap;

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
    contributions::{Contribution, Contributions, Devices, Miner, MinerContributions, PoolMiningEvent, RecentEvents, Winner}, database, error::Error, operator::Operator, tx
};

const MAX_DIFFICULTY: u32 = 22;
const MAX_SCORE: u64 = 2u64.pow(MAX_DIFFICULTY);

/// Aggregates contributions from the pool members.
pub struct Aggregator {
    /// The current challenge.
    pub current_challenge: Challenge,

    /// The set of contributions for attribution.
    pub contributions: Contributions,

    /// The number of workers that have been approved for the current challenge.
    pub num_members: u64,

    /// The set of recent mining events.
    pub recent_events: RecentEvents,
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
            let mut write = aggregator.write().await;
            let contributions = write.get_current_contributions();
            match contributions {
                Ok(contributions) => contributions.total_score,
                Err(err) => {
                    log::error!("{:?}", err);
                    0
                }
            }
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
        
        // build self
        let mut contributions = Contributions::new(15 + 1);
        contributions.insert(challenge.lash_hash_at as u64);
        let aggregator = Aggregator {
            current_challenge: challenge,
            contributions,
            num_members: pool.last_total_members,
            recent_events: RecentEvents::new(15),
        };
        Ok(aggregator)
    }

    fn insert(&mut self, contribution: &mut Contribution) -> Result<(), Error> {
        let challenge = &self.current_challenge.clone();
        let solution = &contribution.solution;

        // normalize contribution score
        let normalized_score = contribution.score.min(MAX_SCORE);
        contribution.score = normalized_score;

        // get current contributions
        let contributions = self.get_current_contributions()?;

        // validate solution against current challenge
        if !drillx::is_valid_digest(&challenge.challenge, &solution.n, &solution.d) {
            log::error!("invalid solution");
            return Err(Error::Internal("invalid solution".to_string()));
        }

        // insert
        let insert = contributions.contributions.replace(*contribution);
        match insert {
            Some(prev) => {
                if contribution.score.gt(&prev.score) {
                    let difficulty = contribution.solution.to_hash().difficulty();
                    let contender = Winner {
                        solution: contribution.solution,
                        difficulty,
                    };
                    // decrement previous score
                    contributions.total_score -= prev.score;

                    // increment new score
                    contributions.total_score += contribution.score;

                    // update winner
                    match contributions.winner {
                        Some(winner) => {
                            if difficulty > winner.difficulty {
                                contributions.winner = Some(contender);
                            }
                        }
                        None => contributions.winner = Some(contender),
                    }
                }
                Ok(())
            }
            None => {
                let difficulty = contribution.solution.to_hash().difficulty();
                let contender = Winner {
                    solution: contribution.solution,
                    difficulty,
                };

                log::info!("new contribution: {:?} {}", contribution.member, difficulty);

                // increment score
                contributions.total_score += contribution.score;

                // update winner
                match contributions.winner {
                    Some(winner) => {
                        if difficulty > winner.difficulty {
                            contributions.winner = Some(contender);
                        }
                    }
                    None => contributions.winner = Some(contender),
                }
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
        let (pool_address, _) = ore_pool_api::state::pool_pda(*authority);
        let (pool_proof_address, _) = ore_pool_api::state::pool_proof_pda(pool_address);
        let bus = self.find_bus(operator).await?;

        // build instructions
        let auth_ix = ore_api::sdk::auth(pool_proof_address);
        let submit_ix = ore_pool_api::sdk::submit(
            operator.keypair.pubkey(),
            best_solution,
            attestation,
            bus,
            vec![], // TODO fetch from reservation accounts
        );
        let rotate_ix = ore_boost_api::sdk::rotate(operator.keypair.pubkey(), pool_proof_address);
        let rpc_client = &operator.rpc_client;
        let sig = tx::submit::submit_instructions(
            &operator.keypair,
            rpc_client,
            &[auth_ix, submit_ix, rotate_ix],
            1_500_000,
            500_000,
        )
        .await?;
        log::info!("{:?}", sig);

        // reset
        self.reset(operator).await?;
        Ok(())
    }

    pub fn get_device_id(&mut self, miner: Miner) -> u8 {
        // get device indices at current challenge
        let last_hash_at = &self.current_challenge.lash_hash_at;
        let all_devices = &mut self.contributions.devices;
        let mut new_devices: Devices = HashMap::new();
        new_devices.insert(miner, 0);
        let current_devices = all_devices
            .entry((*last_hash_at) as u64)
            .or_insert(new_devices);

        // lookup miner device id against current challenge
        let device_id = current_devices.entry(miner).or_insert(0);

        // increment device id
        *device_id += 1;
        *device_id
    }

    pub async fn distribute_rewards(
        &mut self,
        operator: &Operator,
        event: &PoolMiningEvent,
    ) -> Result<(), Error> {
        log::info!("{:?}", event);

        // Compute operator rewards
        let operator_rewards = self.rewards_distribution_operator(
            operator.keypair.pubkey(),
            &event.mine_event,
            operator.operator_commission,
        );

        // Compute miner rewards
        let mut rewards_distribution =
            self.rewards_distribution(&event.mine_event, operator_rewards.1);
        
        println!("rewards_distribution: {:?}", rewards_distribution);

        // Collect all rewards
        rewards_distribution.push(operator_rewards);

        // Write rewards to db
        let mut db_client = operator.db_client.get().await?;
        database::update_member_balances(&mut db_client, rewards_distribution.clone()).await?;

        // Get best member scores for this event
        let member_scores = if let Some(miner_contributions) = self.contributions.miners.get(&(event.mine_event.last_hash_at as u64)) {
            let mut member_scores = HashMap::new();
            for contribution in miner_contributions.contributions.iter() {
                if contribution.score > *member_scores.get(&contribution.member).unwrap_or(&0) {
                    member_scores.insert(contribution.member, contribution.score);
                }
            }
            member_scores
        } else {
            HashMap::new()
        };        

        // Insert record into recent events 
        let mut event = event.clone();
        event.member_scores = member_scores;
        event.member_rewards = HashMap::from_iter(rewards_distribution);
        self.recent_events.insert(
            event.mine_event.last_hash_at as u64,
            event
        );

        Ok(())
    }

    fn rewards_distribution(
        &mut self,
        event: &ore_api::event::MineEvent,
        operator_rewards: u64,
    ) -> Vec<(Pubkey, u64)> {
        // Get attributed scores
        let contributions = &mut self.contributions;
        let (total_score, scores) = contributions.scores();
        
        // Calculate total miner rewards
        let miner_rewards = event.net_reward.checked_sub(operator_rewards).unwrap();
        log::info!("total miner rewards: {}", miner_rewards);

        // compute member split
        scores
            .iter()
            .map(|(member, member_score)| {
                let member_rewards = (miner_rewards as u128)
                    .checked_mul(*member_score as u128)
                    .unwrap()
                    .checked_div(total_score as u128)
                    .unwrap_or(0) as u64;
                (*member, member_rewards)
            })
            .collect()
    }

    fn rewards_distribution_operator(
        &self,
        pool_authority: Pubkey,
        event: &ore_api::event::MineEvent,
        operator_commission: u64,
    ) -> (Pubkey, u64) {
        let operator_rewards = (event.net_reward as u128)
            .saturating_mul(operator_commission as u128)
            .saturating_div(100) as u64;
        log::info!("total operator rewards: {}", operator_rewards);
        (pool_authority, operator_rewards)
    }

    /// fetch the bus with the largest balance
    async fn find_bus(&self, operator: &Operator) -> Result<Pubkey, Error> {
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
        let num_contributions = contributions.contributions.len();
        log::info!("num contributions: {}", num_contributions);
        for contribution in contributions.contributions.iter() {
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
        let last_hash_at = self.current_challenge.lash_hash_at as u64;
        let contributions = &mut self.contributions;
        let contributions = contributions
            .miners
            .get_mut(&last_hash_at)
            .ok_or(Error::Internal(
                "missing contributions at current hash".to_string(),
            ))?;
        Ok(contributions)
    }

    async fn reset(&mut self, operator: &Operator) -> Result<(), Error> {
        log::info!("resetting");

        // update challenge
        self.update_challenge(operator).await?;

        // allocate key for new contributions
        let last_hash_at = self.current_challenge.lash_hash_at as u64;
        let contributions = &mut self.contributions;
        log::info!("new contributions key: {:?}", last_hash_at);
        contributions.insert(last_hash_at);

        // reset accumulators
        let pool = operator.get_pool().await?;
        self.num_members = pool.last_total_members;
        Ok(())
    }

    fn winner(&mut self) -> Result<Winner, Error> {
        let contributions = self.get_current_contributions()?;
        let winner = contributions.winner;
        winner.ok_or(Error::Internal("no solutions were submitted".to_string()))
    }

    async fn check_for_reset(&self, operator: &Operator) -> Result<bool, Error> {
        let last_hash_at = self.current_challenge.lash_hash_at;
        let proof = operator.get_proof().await?;
        let needs_reset = proof.last_hash_at != last_hash_at;
        Ok(needs_reset)
    }

    async fn update_challenge(&mut self, operator: &Operator) -> Result<(), Error> {
        let max_retries = 10;
        let mut retries = 0;
        let last_hash_at = self.current_challenge.lash_hash_at;
        loop {
            let proof = operator.get_proof().await?;
            if proof.last_hash_at != last_hash_at {
                let cutoff_time = operator.get_cutoff(&proof).await?;
                let min_difficulty = operator.min_difficulty().await?;
                self.current_challenge.challenge = proof.challenge;
                self.current_challenge.lash_hash_at = proof.last_hash_at;
                self.current_challenge.min_difficulty = min_difficulty;
                self.current_challenge.cutoff_time = cutoff_time;
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
