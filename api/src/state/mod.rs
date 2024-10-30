mod member;
mod pool;
mod share;

pub use member::*;
pub use pool::*;
pub use share::*;

use steel::*;

use crate::consts::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
pub enum AccountDiscriminator {
    Member = 100,
    Pool = 101,
    Share = 102,
}

pub fn pool_pda(authority: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POOL, authority.as_ref()], &crate::id())
}

pub fn pool_proof_pda(pool: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ore_api::consts::PROOF, pool.as_ref()], &ore_api::id())
}

pub fn pool_pending_stake_token_address(pool: Pubkey, mint: Pubkey) -> Pubkey {
    spl_associated_token_account::get_associated_token_address(&pool, &mint)
}

pub fn pool_stake_pda(pool: Pubkey, mint: Pubkey) -> (Pubkey, u8) {
    let boost_pda = ore_boost_api::state::boost_pda(mint);
    ore_boost_api::state::stake_pda(pool, boost_pda.0)
}

pub fn member_pda(authority: Pubkey, pool: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MEMBER, authority.as_ref(), pool.as_ref()], &crate::id())
}

pub fn share_pda(authority: Pubkey, pool: Pubkey, mint: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SHARE, authority.as_ref(), pool.as_ref(), mint.as_ref()],
        &crate::id(),
    )
}
