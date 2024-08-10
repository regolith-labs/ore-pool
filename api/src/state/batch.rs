use bytemuck::{Pod, Zeroable};

use crate::utils::{impl_account_from_bytes, impl_to_bytes, Discriminator};

use super::AccountDiscriminator;

/// Batch ...
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Batch {
    pub amount: u64,
    pub attestation: [u8; 32],
    pub certification: [u8; 32],
    pub challenge: [u8; 32],
    pub best_solution_digest: [u8; 16],
    pub best_solution_nonce: [u8; 8],
    pub id: u64,
    pub total_solutions: u64,
    pub total_difficulty_score: u64,
}

impl Discriminator for Batch {
    fn discriminator() -> u8 {
        AccountDiscriminator::Batch.into()
    }
}

impl_to_bytes!(Batch);
impl_account_from_bytes!(Batch);
