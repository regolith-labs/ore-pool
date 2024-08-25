use bytemuck::{Pod, Zeroable};
use ore_utils::{impl_account_from_bytes, impl_to_bytes, Discriminator};
use solana_program::pubkey::Pubkey;

use super::AccountDiscriminator;

/// Pool tracks global lifetime stats about the mining pool.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Pool {
    /// The latest attestation posted by this pool operator.
    pub attestation: [u8; 32],

    /// The authority of this pool.
    pub authority: Pubkey,

    /// The total number of members in this pool.
    pub total_members: u64,

    /// The total number of hashes this pool has submitted.
    pub total_submissions: u64,
}

impl Discriminator for Pool {
    fn discriminator() -> u8 {
        AccountDiscriminator::Pool.into()
    }
}

impl_to_bytes!(Pool);
impl_account_from_bytes!(Pool);
