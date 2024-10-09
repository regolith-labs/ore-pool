use steel::*;

use super::AccountDiscriminator;

/// Share tracks a member's contribution to the pool stake account.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Share {
    /// The authority of this share account.
    pub authority: Pubkey,

    /// The stake balance the authority has deposited and may unstake.
    pub balance: u64,

    /// The mint this share account is associated with.
    pub mint: Pubkey,

    /// The pool this share account is associated with.
    pub pool: Pubkey,
}

account!(AccountDiscriminator, Share);
