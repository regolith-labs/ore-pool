use steel::*;

use super::AccountDiscriminator;

/// Member records the participant's claimable balance in the mining pool.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Member {
    /// The member id.
    pub id: u64,

    /// The pool this member belongs to.
    pub pool: Pubkey,

    /// The authority allowed to claim this balance.
    pub authority: Pubkey,

    /// The current balance amount which may be claimed.
    pub balance: u64,

    /// The total balance this member has earned in the lifetime of their participation in the pool.
    pub total_balance: u64,
}

account!(AccountDiscriminator, Member);
