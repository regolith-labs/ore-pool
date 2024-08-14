mod member;
mod pool;
mod submission;

pub use member::*;
pub use pool::*;
use solana_program::pubkey::Pubkey;
pub use submission::*;

use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::consts::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
pub enum AccountDiscriminator {
    Member = 100,
    Pool = 101,
    Submission = 102,
}

pub fn pool_pda(authority: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POOL, authority.as_ref()], &crate::id())
}

pub fn member_pda(authority: Pubkey, pool: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MEMBER, authority.as_ref(), pool.as_ref()], &crate::id())
}

pub fn submission_pda(pool: Pubkey, id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SUBMISSION, pool.as_ref(), id.to_le_bytes().as_slice()],
        &crate::id(),
    )
}
