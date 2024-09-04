use bytemuck::{Pod, Zeroable};
use ore_utils::{account, Discriminator};
use solana_program::pubkey::Pubkey;

use super::AccountDiscriminator;

/// Pool tracks global lifetime stats about the mining pool.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Pool {
    /// The authority of this pool.
    pub authority: Pubkey,

    /// The url where hashes should be submitted (right padded with 0s).
    pub url: [u8; 128],

    /// The latest attestation posted by this pool operator.
    pub attestation: [u8; 32],

    /// Foreign key to the ORE proof account.
    pub last_hash_at: i64,

    /// The total number of hashes this pool has submitted.
    pub total_submissions: u64,

    /// The total number of members in this pool.
    pub total_members: u64,

    // The total number of members in this pool at the last submission.
    pub last_total_members: u64,
}

impl Discriminator for Pool {
    fn discriminator() -> u8 {
        AccountDiscriminator::Pool.into()
    }
}

account!(Pool);
