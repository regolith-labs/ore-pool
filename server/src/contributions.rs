use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;

use drillx::Solution;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;

pub struct Contributions {
    pub miners: HashMap<LastHashAt, MinerContributions>,
    attribution_filter: AttributionFilter,
}

impl Contributions {
    pub fn new(attribution_filter_size: u8) -> Self {
        Self {
            miners: HashMap::new(),
            attribution_filter: AttributionFilter::new(attribution_filter_size),
        }
    }

    pub fn insert(&mut self, ts: LastHashAt) {
        let contributions = MinerContributions {
            contributions: HashSet::new(),
            winner: None,
            total_score: 0,
        };
        if self.miners.insert(ts, contributions).is_some() {
            log::error!("contributions at last-hash-at already exist: {}", ts);
        }
        self.attribution_filter.push(ts);
    }

    pub fn scores(&mut self) -> (TotalScore, Vec<(Miner, u64)>) {
        // filter for valid timestamps
        self.filter();

        // sum contribution scores for each member
        let mut total_score: u64 = 0;
        let mut merge: HashMap<Miner, u64> = HashMap::new();
        let duplicates = self
            .miners
            .values()
            .flat_map(|mc| mc.contributions.iter())
            .map(|c| (c.member, c.score));
        for (k, v) in duplicates {
            *merge.entry(k).or_insert(0) += v;
            total_score += v;
        }
        (total_score, merge.into_iter().collect())
    }

    fn filter(&mut self) {
        let validation = &self.attribution_filter.time_stamps;
        self.miners.retain(|k, _v| validation.contains(k));
    }
}

/// miner authority
pub type Miner = Pubkey;

/// miner lookup table
/// challenge --> contribution
pub type LastHashAt = u64;
pub struct MinerContributions {
    pub contributions: HashSet<Contribution>,
    pub winner: Option<Winner>,
    pub total_score: u64,
}

/// total score per challenge
pub type TotalScore = u64;

/// timestamps of last n challenges
pub struct AttributionFilter {
    /// total num elements in vec
    pub len: u8,
    /// max size of vec
    pub size: u8,
    /// elements
    pub time_stamps: VecDeque<LastHashAt>,
}

impl AttributionFilter {
    pub fn new(size: u8) -> Self {
        Self {
            len: 0,
            size,
            time_stamps: VecDeque::with_capacity(size as usize),
        }
    }
    pub fn push(&mut self, ts: LastHashAt) {
        if self.len.lt(&self.size) {
            self.len += 1;
            self.time_stamps.push_back(ts);
        } else {
            self.time_stamps.pop_front();
            self.time_stamps.push_back(ts);
        }
    }
}

/// Best hash to be submitted for the current challenge.
#[derive(Clone, Copy, Debug)]
pub struct Winner {
    /// The winning solution.
    pub solution: Solution,

    /// The current largest difficulty.
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

/// Tracks recent mining events and rewards for each submission
pub struct RecentEvents {
    /// Maps last_hash_at timestamp to mining event data
    events: HashMap<LastHashAt, PoolMiningEvent>,
    /// Maximum number of events to keep in memory
    max_events: usize,
}

#[derive(Clone, Debug)]
pub struct PoolMiningEvent {
    pub signature: Signature,
    pub block: u64,
    pub timestamp: u64,
    pub mine_event: ore_api::event::MineEvent,
    pub member_rewards: HashMap<Pubkey, u64>,
    pub member_scores: HashMap<Pubkey, u64>,
}

impl RecentEvents {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: HashMap::with_capacity(max_events),
            max_events,
        }
    }

    pub fn keys(&self) -> Vec<LastHashAt> {
        self.events.keys().cloned().collect()
    }

    pub fn insert(&mut self, last_hash_at: LastHashAt, event: PoolMiningEvent) {
        if self.events.len() >= self.max_events {
            // Remove oldest event if at capacity
            if let Some(oldest) = self.events.keys().min().copied() {
                self.events.remove(&oldest);
            }
        }
        self.events.insert(last_hash_at, event);
    }

    pub fn get(&self, last_hash_at: LastHashAt) -> Option<&PoolMiningEvent> {
        self.events.get(&last_hash_at)
    }
}
