use steel::*;

use super::AccountDiscriminator;

/// Tracks the lifetime rewards distributed in the pool amongst all the parties.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct TotalRewards {
    /// The pool the total rewards accounts are associated with.
    pub pool: Pubkey,

    /// The total lifetime rewards distributed to miners in the pool.
    pub miner_rewards: u64,

    /// The total lifetime rewards distributed to stakers in the pool.
    pub staker_rewards: u64,

    /// The total lifetime rewards distributed to the pool operator.
    pub operator_rewards: u64,
}

/// Tracks the lifetime rewards distributed to share holders of a particular boost account.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct ShareRewards {
    /// The pool the share accounts are associated with.
    pub pool: Pubkey,

    /// The mint the share accounts are associated with.
    pub mint: Pubkey,

    /// The total rewards attributed to the share holders.
    pub rewards: u64,
}

account!(AccountDiscriminator, TotalRewards);
account!(AccountDiscriminator, ShareRewards);
