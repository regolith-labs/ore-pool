use drillx::Solution;
use ore_api::{
    consts::{CONFIG_ADDRESS, TREASURY_ADDRESS, TREASURY_TOKENS_ADDRESS},
    state::proof_pda,
};
use steel::*;

use crate::{
    consts::LEGACY_BOOST_PROGRAM_ID,
    error::ApiError,
    instruction::*,
    state::{legacy_boost_pda, legacy_stake_pda, member_pda, pool_pda, pool_proof_pda, share_pda},
};

/// Builds a launch instruction.
#[allow(deprecated)]
pub fn launch(signer: Pubkey, miner: Pubkey, url: String) -> Result<Instruction, ApiError> {
    let url = url_to_bytes(url.as_str())?;
    let (pool_pda, pool_bump) = pool_pda(signer);
    let (proof_pda, proof_bump) = pool_proof_pda(pool_pda);
    let ix = Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(miner, false),
            AccountMeta::new(pool_pda, false),
            AccountMeta::new(proof_pda, false),
            AccountMeta::new_readonly(ore_api::ID, false),
            AccountMeta::new_readonly(ore_boost_api::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(sysvar::slot_hashes::ID, false),
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

/// Builds an join instruction.
#[allow(deprecated)]
pub fn join(member_authority: Pubkey, pool: Pubkey, payer: Pubkey) -> Instruction {
    let (member_pda, member_bump) = member_pda(member_authority, pool);
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(member_authority, false),
            AccountMeta::new(member_pda, false),
            AccountMeta::new(pool, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: Join { member_bump }.to_bytes(),
    }
}

/// Builds a claim instruction.
#[allow(deprecated)]
pub fn claim(
    signer: Pubkey,
    beneficiary: Pubkey,
    pool_address: Pubkey,
    amount: u64,
) -> Instruction {
    let (member_pda, _) = member_pda(signer, pool_address);
    let (pool_proof_pda, _) = proof_pda(pool_address);
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(beneficiary, false),
            AccountMeta::new(member_pda, false),
            AccountMeta::new(pool_address, false),
            AccountMeta::new(pool_proof_pda, false),
            AccountMeta::new_readonly(TREASURY_ADDRESS, false),
            AccountMeta::new(TREASURY_TOKENS_ADDRESS, false),
            AccountMeta::new_readonly(ore_api::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
        ],
        data: Claim {
            amount: amount.to_le_bytes(),
            pool_bump: 0,
        }
        .to_bytes(),
    }
}

/// Builds an attribute instruction.
pub fn attribute(signer: Pubkey, member_authority: Pubkey, total_balance: u64) -> Instruction {
    let (pool_pda, _) = pool_pda(signer);
    let (proof_pda, _) = pool_proof_pda(pool_pda);
    let (member_pda, _) = member_pda(member_authority, pool_pda);
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(pool_pda, false),
            AccountMeta::new_readonly(proof_pda, false),
            AccountMeta::new(member_pda, false),
        ],
        data: Attribute {
            total_balance: total_balance.to_le_bytes(),
        }
        .to_bytes(),
    }
}

/// Builds a commit instruction.
#[deprecated(
    since = "0.3.0",
    note = "Staking has moved to the global boost program"
)]
#[allow(deprecated)]
pub fn commit(_signer: Pubkey, _mint: Pubkey) -> Instruction {
    panic!("Staking has moved to the global boost program");
}

/// Builds an submit instruction.
pub fn submit(
    signer: Pubkey,
    solution: Solution,
    attestation: [u8; 32],
    bus: Pubkey,
    boost_accounts: Option<[Pubkey; 3]>,
) -> Instruction {
    let (pool_pda, _) = pool_pda(signer);
    let (proof_pda, _) = pool_proof_pda(pool_pda);
    let mut accounts = vec![
        AccountMeta::new(signer, true),
        AccountMeta::new(bus, false),
        AccountMeta::new_readonly(CONFIG_ADDRESS, false),
        AccountMeta::new(pool_pda, false),
        AccountMeta::new(proof_pda, false),
        AccountMeta::new_readonly(ore_api::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(sysvar::instructions::ID, false),
        AccountMeta::new_readonly(sysvar::slot_hashes::ID, false),
    ];
    if let Some(boost_accounts) = boost_accounts {
        accounts.push(AccountMeta::new_readonly(boost_accounts[0], false));
        accounts.push(AccountMeta::new(boost_accounts[1], false));
        accounts.push(AccountMeta::new_readonly(boost_accounts[2], false));
    }
    Instruction {
        program_id: crate::ID,
        accounts,
        data: Submit {
            attestation,
            digest: solution.d,
            nonce: solution.n,
        }
        .to_bytes(),
    }
}

/// Builds an unstake instruction for legacy boost program.
pub fn unstake(
    signer: Pubkey,
    mint: Pubkey,
    pool: Pubkey,
    recipient: Pubkey,
    amount: u64,
) -> Instruction {
    let (boost_pda, _) = legacy_boost_pda(mint);
    let boost_tokens =
        spl_associated_token_account::get_associated_token_address(&boost_pda, &mint);
    let (member_pda, _) = member_pda(signer, pool);
    let pool_tokens = spl_associated_token_account::get_associated_token_address(&pool, &mint);
    let (share_pda, _) = share_pda(signer, pool, mint);
    let (stake_pda, _) = legacy_stake_pda(pool, boost_pda);
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(boost_pda, false),
            AccountMeta::new(boost_tokens, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(member_pda, false),
            AccountMeta::new(pool, false),
            AccountMeta::new(pool_tokens, false),
            AccountMeta::new(recipient, false),
            AccountMeta::new(share_pda, false),
            AccountMeta::new(stake_pda, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(LEGACY_BOOST_PROGRAM_ID, false),
        ],
        data: Unstake {
            amount: amount.to_le_bytes(),
        }
        .to_bytes(),
    }
}

/// builds a stake instruction.
#[deprecated(
    since = "0.3.0",
    note = "Staking has moved to the global boost program"
)]
#[allow(deprecated)]
pub fn stake(
    signer: Pubkey,
    mint: Pubkey,
    pool: Pubkey,
    sender: Pubkey,
    amount: u64,
) -> Instruction {
    let (member_pda, _) = member_pda(signer, pool);
    let pool_tokens = spl_associated_token_account::get_associated_token_address(&pool, &mint);
    let (share_pda, _) = share_pda(signer, pool, mint);
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(member_pda, false),
            AccountMeta::new_readonly(pool, false),
            AccountMeta::new(pool_tokens, false),
            AccountMeta::new(sender, false),
            AccountMeta::new(share_pda, false),
            AccountMeta::new_readonly(spl_token::ID, false),
        ],
        data: Stake {
            amount: amount.to_le_bytes(),
        }
        .to_bytes(),
    }
}

/// Builds an open share instruction.
#[deprecated(
    since = "0.3.0",
    note = "Staking has moved to the global boost program"
)]
#[allow(deprecated)]
pub fn open_share(_signer: Pubkey, _mint: Pubkey, _pool: Pubkey) -> Instruction {
    panic!("Staking has moved to the global boost program");
}

/// Builds an open stake instruction.
#[deprecated(
    since = "0.3.0",
    note = "Staking has moved to the global boost program"
)]
#[allow(deprecated)]
pub fn open_stake(_signer: Pubkey, _mint: Pubkey) -> Instruction {
    panic!("Staking has moved to the global boost program");
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
