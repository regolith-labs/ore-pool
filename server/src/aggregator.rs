use std::{collections::HashSet, hash::Hash};

use drillx::Solution;
use sha3::{Digest, Sha3_256};
use solana_sdk::{pubkey::Pubkey, signer::Signer};

use crate::{error::Error, operator::Operator, tx};

/// Aggregates contributions from the pool members.
pub struct Aggregator {
    // The current challenge.
    pub challenge: Challenge,

    /// The set of contributions aggregated for the current challenge.
    pub contributions: HashSet<Contribution>,

    /// The total difficulty score of all the contributions aggregated so far.
    pub total_score: u64,

    // The best solution submitted.
    pub winner: Option<Winner>,
}

pub struct Challenge {
    /// The current challenge the pool is accepting solutions for.
    pub challenge: [u8; 32],

    /// Foreign key to the ORE proof account.
    pub lash_hash_at: i64,

    // The current minimum difficulty accepted by the ORE program.
    pub min_difficulty: u64,

    // The cutoff time to stop accepting contributions.
    pub cutoff_time: u64,
}

// Best hash to be submitted for the current challenge.
#[derive(Clone, Copy)]
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
    aggregator: &tokio::sync::Mutex<Aggregator>,
    operator: &Operator,
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<Contribution>,
) -> Result<(), Error> {
    let mut timer = tokio::time::Instant::now();
    loop {
        while let Some(contribution) = rx.recv().await {
            let mut aggregator = aggregator.lock().await;
            let cutoff_time = aggregator.challenge.cutoff_time;
            let out_of_time = timer.elapsed().as_secs().ge(&cutoff_time);
            if out_of_time {
                aggregator.submit_and_reset(operator, &mut timer).await?;
            } else {
                aggregator.insert(&contribution)
            }
        }
    }
}

impl Aggregator {
    pub async fn new(operator: &Operator) -> Result<Self, Error> {
        let proof = operator.get_proof().await?;
        let config = operator.get_config().await?;
        let cutoff_time = operator.get_cutoff(&proof).await?;
        let challenge = Challenge {
            challenge: proof.challenge,
            lash_hash_at: proof.last_hash_at,
            min_difficulty: config.min_difficulty,
            cutoff_time,
        };
        let aggregator = Aggregator {
            challenge,
            contributions: HashSet::new(),
            total_score: 0,
            winner: None,
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

    async fn submit_and_reset(
        &mut self,
        operator: &Operator,
        timer: &mut tokio::time::Instant,
    ) -> Result<(), Error> {
        let best_solution = self.winner()?.solution;
        let attestation = self.attestation();
        let ix = ore_pool_api::instruction::submit(
            operator.keypair.pubkey(),
            best_solution,
            attestation,
        );
        let rpc_client = &operator.rpc_client;
        let sig = tx::submit(&operator.keypair, rpc_client, vec![ix], 1_000_000, 500_000).await?;
        log::info!("{:?}", sig);

        // TODO Parse tx response
        // TODO Update members' local attribution balances
        // TODO Publish block to S3

        self.reset(operator, timer).await?;
        Ok(())
    }

    fn attestation(&self) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        let contributions = &self.contributions;
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

    async fn reset(
        &mut self,
        operator: &Operator,
        timer: &mut tokio::time::Instant,
    ) -> Result<(), Error> {
        self.update_challenge(operator).await?;
        self.contributions = HashSet::new();
        self.total_score = 0;
        self.winner = None;
        *timer = tokio::time::Instant::now();
        Ok(())
    }

    fn winner(&self) -> Result<Winner, Error> {
        self.winner
            .ok_or(Error::Internal("no solutions were submitted".to_string()))
    }

    async fn update_challenge(&mut self, operator: &Operator) -> Result<(), Error> {
        let max_retries = 5;
        let mut retries = 0;
        let last_hash_at = self.challenge.lash_hash_at;
        loop {
            let proof = operator.get_proof().await?;
            if proof.last_hash_at != last_hash_at {
                let config = operator.get_config().await?;
                let cutoff_time = operator.get_cutoff(&proof).await?;
                self.challenge.challenge = proof.challenge;
                self.challenge.lash_hash_at = proof.last_hash_at;
                self.challenge.min_difficulty = config.min_difficulty;
                self.challenge.cutoff_time = cutoff_time;
                return Ok(());
            } else {
                retries += 1;
                if retries == max_retries {
                    return Err(Error::Internal("failed to fetch new challenge".to_string()));
                }
            }
        }
    }
}
