use bytemuck::{Pod, Zeroable};
use solana_program::pubkey::Pubkey;

use crate::utils::{impl_account_from_bytes, impl_to_bytes, Discriminator};

use super::AccountDiscriminator;

/// Member records the participant's claimable balance in the mining pool.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Member {
    /// Whether or not the member is approved to participate in the pool.
    pub approved: u64,

    /// The authority allowed to claim this balance.
    pub authority: Pubkey,

    /// The balance amount which may be claimed.
    pub balance: u64,

    /// The member id.
    pub id: u64,

    /// The pool this member belongs to.
    pub pool: Pubkey,

    /// The total amount this member has earned in the lifetime of their account.
    pub total_balance: u64,
}

impl Discriminator for Member {
    fn discriminator() -> u8 {
        AccountDiscriminator::Member.into()
    }
}

impl_to_bytes!(Member);
impl_account_from_bytes!(Member);
