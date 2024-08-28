use bytemuck::{Pod, Zeroable};
use drillx::Solution;
use num_enum::TryFromPrimitive;
use ore_utils::instruction;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
    sysvar::slot_hashes,
};

use crate::{
    error::ApiError,
    state::{member_pda, pool_pda, pool_proof_pda},
};

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
#[rustfmt::skip]
pub enum PoolInstruction {
    // User
    Open = 0,
    Claim = 1,
    
    // Operator
    Attribute = 100,
    Launch = 101,
    Submit = 102,
}

impl PoolInstruction {
    pub fn to_vec(&self) -> Vec<u8> {
        vec![*self as u8]
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct AttributeArgs {
    pub total_balance: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ClaimArgs {
    pub amount: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct LaunchArgs {
    pub pool_bump: u8,
    pub proof_bump: u8,
    pub url: [u8; 128],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct OpenArgs {
    pub member_bump: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SubmitArgs {
    pub attestation: [u8; 32],
    pub digest: [u8; 16],
    pub nonce: [u8; 8],
}

instruction!(LaunchArgs);
instruction!(ClaimArgs);
instruction!(AttributeArgs);
instruction!(OpenArgs);
instruction!(SubmitArgs);

/// Builds a launch instruction.
pub fn launch(signer: Pubkey, miner: Pubkey, url: String) -> Result<Instruction, ApiError> {
    let url = url_to_bytes(url.as_str())?;
    let (pool_pda, pool_bump) = pool_pda(signer);
    let (proof_pda, proof_bump) = pool_proof_pda(pool_pda);
    let ix = Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(miner, false),
            AccountMeta::new(pool_pda, false),
            AccountMeta::new(proof_pda, false),
            AccountMeta::new_readonly(ore_api::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(slot_hashes::id(), false),
        ],
        data: [
            PoolInstruction::Launch.to_vec(),
            LaunchArgs {
                pool_bump,
                proof_bump,
                url,
            }
            .to_bytes()
            .to_vec(),
        ]
        .concat(),
    };
    Ok(ix)
}

fn url_to_bytes(input: &str) -> Result<[u8; 128], ApiError> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    if len > 128 {
        Err(ApiError::UrlTooLarge)
    } else {
        let mut array = [0u8; 128];
        array[..len].copy_from_slice(&bytes[..len]);
        Ok(array)
    }
}

pub fn open(signer: Pubkey, pool: Pubkey) -> Instruction {
    let (member_pda, member_bump) = member_pda(signer, pool);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(member_pda, false),
            AccountMeta::new(pool, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: [
            PoolInstruction::Open.to_vec(),
            OpenArgs { member_bump }.to_bytes().to_vec(),
        ]
        .concat(),
    }
}

/// Builds an submit instruction.
pub fn submit(signer: Pubkey, solution: Solution, attestation: [u8; 32]) -> Instruction {
    Instruction {
        program_id: crate::id(),
        accounts: vec![AccountMeta::new(signer, true)],
        data: [
            PoolInstruction::Submit.to_vec(),
            SubmitArgs {
                attestation,
                digest: solution.d,
                nonce: solution.n,
            }
            .to_bytes()
            .to_vec(),
        ]
        .concat(),
    }
}
