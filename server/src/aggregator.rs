use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    hash::Hash,
};

use drillx::Solution;
use sha3::{Digest, Sha3_256};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use types::{Challenge, MemberChallenge};

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

    // The challenge coordinator that issues nonce indices to participating members.
    pub coordinator: ChallengeCoordinator,
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

pub struct ChallengeCoordinator {
    // The number of total members that could participate in this challenge loop
    // locked per "epoch" to determinstically distribute nonce indices
    pub num_total_members_limit: u64,

    // The sequentially incremented value member by member who fetches challenge.
    pub num_total_members: u64,

    // The lookup map from member authority to nonce index
    // coordinated per epoch
    pub nonce_indices: HashMap<Pubkey, u64>,
}

impl ChallengeCoordinator {
    async fn new() -> Result<Self, Error> {
        let num_total_members_limit = num_total_members_limit().await?;
        let coordinator = ChallengeCoordinator {
            num_total_members_limit,
            num_total_members: 0,
            nonce_indices: HashMap::new(),
        };
        Ok(coordinator)
    }
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
                if let Err(err) = aggregator.submit_and_reset(operator, &mut timer).await {
                    // keep server looping
                    // there may be scenarios where the server doesn't receive solutions, etc.
                    log::error!("{:?}", err);
                }
            } else {
                aggregator.insert(&contribution)
            }
        }
    }
}

// TODO: persist / lookup
async fn num_total_members_limit() -> Result<u64, Error> {
    Ok(100)
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
        let coordinator = ChallengeCoordinator::new().await?;
        let aggregator = Aggregator {
            challenge,
            contributions: HashSet::new(),
            total_score: 0,
            winner: None,
            coordinator,
        };
        Ok(aggregator)
    }

    // TODO: error if member not approved
    pub async fn nonce_index(
        &mut self,
        member_authority: &Pubkey,
    ) -> Result<MemberChallenge, Error> {
        let challenge = &self.challenge;
        let num_total_members_limit = self.coordinator.num_total_members_limit;
        let mut num_total_members = self.coordinator.num_total_members;
        let coordinator = &mut self.coordinator.nonce_indices;
        let entry = coordinator.entry(*member_authority);
        match entry {
            Entry::Occupied(nonce_index) => Ok(MemberChallenge {
                challenge: *challenge,
                nonce_index: *(nonce_index.get()),
                num_total_members: num_total_members_limit,
            }),
            Entry::Vacant(vacant) => {
                num_total_members += 1;
                vacant.insert(num_total_members);
                Ok(MemberChallenge {
                    challenge: *challenge,
                    nonce_index: num_total_members,
                    num_total_members: num_total_members_limit,
                })
            }
        }
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
        let coordinator = ChallengeCoordinator::new().await?;
        self.coordinator = coordinator;
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
