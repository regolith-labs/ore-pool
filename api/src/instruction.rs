use bytemuck::{Pod, Zeroable};
use num_enum::TryFromPrimitive;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use crate::utils::{impl_instruction_from_bytes, impl_to_bytes};

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
#[rustfmt::skip]
pub enum PoolInstruction {
    // User
    Open = 0,
    Claim = 1,
    Stake = 2,
    
    // Admin
    Certify = 100,
    Commit = 101,
    Initialize = 102,
    Submit = 103
}

impl PoolInstruction {
    pub fn to_vec(&self) -> Vec<u8> {
        vec![*self as u8]
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ClaimArgs {
    pub amount: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CertifyArgs {
    pub digest: [u8; 16],
    pub nonce: [u8; 8],
    pub signature: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CommitArgs {
    pub index: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct InitializeArgs {
    pub pool_bump: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SubmitArgs {
    pub attestation: [u8; 32],
    pub batch_bump: u8,
    pub digest: [u8; 16],
    pub nonce: [u8; 8],
}

impl_to_bytes!(CertifyArgs);
impl_to_bytes!(InitializeArgs);
impl_to_bytes!(CommitArgs);
impl_to_bytes!(SubmitArgs);

impl_instruction_from_bytes!(CertifyArgs);
impl_instruction_from_bytes!(InitializeArgs);
impl_instruction_from_bytes!(CommitArgs);
impl_instruction_from_bytes!(SubmitArgs);

/// Builds an initialize instruction.
pub fn initialize(signer: Pubkey) -> Instruction {
    Instruction {
        program_id: crate::id(),
        accounts: vec![AccountMeta::new(signer, true)],
        data: [PoolInstruction::Initialize.to_vec()].concat(),
    }
}
