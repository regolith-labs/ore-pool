use bytemuck::{Pod, Zeroable};
use drillx::Solution;
use num_enum::TryFromPrimitive;
use ore_api::consts::{CONFIG_ADDRESS, TREASURY_ADDRESS, TREASURY_TOKENS_ADDRESS};
use ore_utils::instruction;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
    sysvar::{instructions, slot_hashes},
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
    Join = 0,
    Claim = 1,

    // Operator
    Attribute = 100,
    Launch = 101,
    Submit = 102,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Attribute {
    pub total_balance: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Claim {
    pub amount: [u8; 8],
    pub pool_bump: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Launch {
    pub pool_bump: u8,
    pub proof_bump: u8,
    pub url: [u8; 128],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Join {
    pub member_bump: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Submit {
    pub attestation: [u8; 32],
    pub digest: [u8; 16],
    pub nonce: [u8; 8],
}

instruction!(PoolInstruction, Launch);
instruction!(PoolInstruction, Claim);
instruction!(PoolInstruction, Attribute);
instruction!(PoolInstruction, Join);
instruction!(PoolInstruction, Submit);

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
        data: Launch {
            pool_bump,
            proof_bump,
            url,
        }
        .to_bytes(),
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

/// Builds an join instruction.
pub fn join(member_authority: Pubkey, pool: Pubkey, payer: Pubkey) -> Instruction {
    let (member_pda, member_bump) = member_pda(member_authority, pool);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(member_authority, false),
            AccountMeta::new(member_pda, false),
            AccountMeta::new(pool, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: Join { member_bump }.to_bytes(),
    }
}

/// Builds a claim instruction.
pub fn claim(
    signer: Pubkey,
    beneficiary: Pubkey,
    pool_pda: Pubkey,
    pool_bump: u8,
    amount: u64,
) -> Instruction {
    let (member_pda, _) = member_pda(signer, pool_pda);
    let (pool_proof_pda, _) = pool_proof_pda(pool_pda);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(beneficiary, false),
            AccountMeta::new(member_pda, false),
            AccountMeta::new(pool_pda, false),
            AccountMeta::new(pool_proof_pda, false),
            AccountMeta::new_readonly(TREASURY_ADDRESS, false),
            AccountMeta::new(TREASURY_TOKENS_ADDRESS, false),
            AccountMeta::new_readonly(ore_api::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: Claim {
            amount: amount.to_le_bytes(),
            pool_bump,
        }
        .to_bytes(),
    }
}

/// Builds an attribute instruction.
pub fn attribute(signer: Pubkey, member_authority: Pubkey, total_balance: u64) -> Instruction {
    let (pool_pda, _) = pool_pda(signer);
    let (member_pda, _) = member_pda(member_authority, pool_pda);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(pool_pda, false),
            AccountMeta::new(member_pda, false),
        ],
        data: Attribute {
            total_balance: total_balance.to_le_bytes(),
        }
        .to_bytes(),
    }
}

/// Builds an submit instruction.
pub fn submit(
    signer: Pubkey,
    solution: Solution,
    attestation: [u8; 32],
    bus: Pubkey,
) -> Instruction {
    let (pool_pda, _) = pool_pda(signer);
    println!("pool pda ix builder: {:?}", pool_pda);
    let (proof_pda, _) = pool_proof_pda(pool_pda);
    println!("proof pda ix builder: {:?}", proof_pda);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(bus, false),
            AccountMeta::new_readonly(CONFIG_ADDRESS, false),
            AccountMeta::new(pool_pda, false),
            AccountMeta::new(proof_pda, false),
            AccountMeta::new_readonly(ore_api::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(instructions::id(), false),
            AccountMeta::new_readonly(slot_hashes::id(), false),
        ],
        data: Submit {
            attestation,
            digest: solution.d,
            nonce: solution.n,
        }
        .to_bytes(),
    }
}
