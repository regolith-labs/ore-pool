use bytemuck::{Pod, Zeroable};

use crate::utils::{impl_account_from_bytes, impl_to_bytes, Discriminator};

use super::AccountDiscriminator;

/// Member ...
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Member {
    pub authority: Pubkey,
    pub balance: u64,
}

impl Discriminator for Member {
    fn discriminator() -> u8 {
        AccountDiscriminator::Member.into()
    }
}

impl_to_bytes!(Member);
impl_account_from_bytes!(Member);
