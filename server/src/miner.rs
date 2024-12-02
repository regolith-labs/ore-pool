use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;

use drillx::Solution;
use solana_sdk::pubkey::Pubkey;

pub struct Miners {
    pub miners: HashMap<LastHashAt, MinerContributions>,
    pub devices: HashMap<LastHashAt, Devices>,
    attribution_filter: AttributionFilter,
}

impl Miners {
    pub fn new(attribution_filter_size: u8) -> Self {
        Self {
            miners: HashMap::new(),
            devices: HashMap::new(),
            attribution_filter: AttributionFilter::new(attribution_filter_size),
        }
    }

    pub fn insert(&mut self, ts: LastHashAt) {
        let contributions = MinerContributions {
            contributions: HashSet::new(),
            winner: None,
            total_score: 0,
        };
        if let Some(_) = self.miners.insert(ts, contributions) {
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
        self.devices.retain(|k, _v| validation.contains(k));
    }
}

/// miner authority
pub type Miner = Pubkey;

/// miner devices
pub type DeviceIndex = u8;
pub type Devices = HashMap<Miner, DeviceIndex>;

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
