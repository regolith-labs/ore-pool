use bytemuck::{Pod, Zeroable};

use crate::utils::{impl_account_from_bytes, impl_to_bytes, Discriminator};

use super::AccountDiscriminator;

/// Submission records a specific submission by the pool operator to the ORE mining contract.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Submission {
    pub amount: u64,
    pub attestation: [u8; 32],
    pub id: u64,
    // pub total_solutions: u64,
    // pub total_difficulty_score: u64,
}

impl Discriminator for Submission {
    fn discriminator() -> u8 {
        AccountDiscriminator::Submission.into()
    }
}

impl_to_bytes!(Submission);
impl_account_from_bytes!(Submission);
