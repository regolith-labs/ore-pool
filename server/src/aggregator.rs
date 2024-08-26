use std::{collections::HashSet, hash::Hash};

use drillx::Solution;
use sha3::{Digest, Sha3_256};
use solana_sdk::{compute_budget::ComputeBudgetInstruction, pubkey::Pubkey, signer::Signer};

use crate::{error::Error, operator::Operator};

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

    /// The last time this account provided a hash.
    /// Relation to the ORE proof account.
    pub lash_hash_at: i64,
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

impl Aggregator {
    pub async fn new(operator: &Operator) -> Result<Self, Error> {
        let proof = operator.get_proof().await?;
        let challenge = Challenge {
            challenge: proof.challenge,
            lash_hash_at: proof.last_hash_at,
        };
        let aggregator = Aggregator {
            challenge,
            contributions: HashSet::new(),
            total_score: 0,
            winner: None,
        };
        Ok(aggregator)
    }

    pub async fn submit(&mut self, operator: &Operator) -> Result<(), Error> {
        // Best solution
        let best_solution = self.winner()?.solution;
        // Generate attestation
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

        // Generate attestation
        let mut attestation: [u8; 32] = [0; 32];
        attestation.copy_from_slice(&hasher.finalize()[..]);

        // TODO Submit best hash and attestation to Solana
        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(600_000);
        let compute_price_ix = ComputeBudgetInstruction::set_compute_unit_price(100_000); // TODO
        let ix = ore_pool_api::instruction::submit(
            operator.keypair.pubkey(),
            best_solution,
            attestation,
        );

        // TODO Parse tx response
        // TODO Update members' local attribution balances
        // TODO Publish block to S3
        // TODO Refresh the challenge

        // Reset the aggregator
        self.reset(operator).await?;
        Ok(())
    }

    async fn reset(&mut self, operator: &Operator) -> Result<(), Error> {
        self.update_challenge(operator).await?;
        self.contributions = HashSet::new();
        self.total_score = 0;
        self.winner = None;
        Ok(())
    }

    fn insert(&mut self, contribution: &Contribution, winner: &mut Winner) {
        match self.contributions.insert(*contribution) {
            true => {
                let difficulty = contribution.solution.to_hash().difficulty();
                if difficulty > winner.difficulty {
                    *winner = Winner {
                        solution: contribution.solution,
                        difficulty,
                    };
                }
                self.total_score += contribution.score;
            }
            false => {
                log::error!("already received contribution: {:?}", contribution.member);
            }
        }
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
                self.challenge.challenge = proof.challenge;
                self.challenge.lash_hash_at = proof.last_hash_at;
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
