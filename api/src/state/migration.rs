use steel::*;

use super::AccountDiscriminator;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Migration {
    pub pool: Pubkey,
    pub members_migrated: u64,
}

account!(AccountDiscriminator, Migration);
