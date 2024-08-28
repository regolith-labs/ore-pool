use bytemuck::{Pod, Zeroable};
use ore_utils::{account, Discriminator};
use solana_program::pubkey::Pubkey;

use super::AccountDiscriminator;

/// Member records the participant's claimable balance in the mining pool.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Member {
    /// The authority allowed to claim this balance.
    pub authority: Pubkey,

    /// The current balance amount which may be claimed.
    pub balance: u64,

    /// The pool this member belongs to.
    pub pool: Pubkey,

    /// The total balance this member has earned in the lifetime of their participation in the pool.
    pub total_balance: u64,
}

impl Discriminator for Member {
    fn discriminator() -> u8 {
        AccountDiscriminator::Member.into()
    }
}

account!(Member);
