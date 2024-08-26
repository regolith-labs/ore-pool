mod member;
mod pool;

pub use member::*;
pub use pool::*;
use solana_program::pubkey::Pubkey;

use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::consts::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
pub enum AccountDiscriminator {
    Member = 100,
    Pool = 101,
}

pub fn pool_pda(authority: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POOL, authority.as_ref()], &crate::id())
}

pub fn pool_proof_pda(pool: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ore_api::consts::PROOF, pool.as_ref()], &ore_api::id())
}

pub fn member_pda(authority: Pubkey, pool: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MEMBER, authority.as_ref(), pool.as_ref()], &crate::id())
}
