use steel::*;

use super::AccountDiscriminator;

/// Pool tracks global lifetime stats about the mining pool.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Pool {
    /// The authority of this pool.
    pub authority: Pubkey,

    /// The bump used for signing CPIs.
    #[deprecated(since = "0.1.3", note = "Bumps are no longer required")]
    pub bump: u64,

    /// The url where hashes should be submitted (right padded with 0s).
    pub url: [u8; 128],

    /// The latest attestation posted by this pool operator.
    pub attestation: [u8; 32],

    /// Foreign key to the ORE proof account.
    pub last_hash_at: i64,

    /// The total claimable rewards in the pool.
    pub total_rewards: u64,

    /// The total number of hashes this pool has submitted.
    pub total_submissions: u64,

    /// The total number of members in this pool.
    pub total_members: u64,

    /// The total number of members in this pool at the last submission.
    pub last_total_members: u64,
}

account!(AccountDiscriminator, Pool);
