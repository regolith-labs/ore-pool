use bytemuck::{Pod, Zeroable};

use crate::utils::{impl_account_from_bytes, impl_to_bytes, Discriminator};

use super::AccountDiscriminator;

/// Member records the participant's claimable balance in the mining pool.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Member {
    /// The authority allowed to claim this balance.
    pub authority: Pubkey,

    /// The balance amount which may be claimed.
    pub balance: u64,
}

impl Discriminator for Member {
    fn discriminator() -> u8 {
        AccountDiscriminator::Member.into()
    }
}

impl_to_bytes!(Member);
impl_account_from_bytes!(Member);
