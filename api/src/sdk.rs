use drillx::Solution;
use ore_api::consts::{CONFIG_ADDRESS, TREASURY_ADDRESS, TREASURY_TOKENS_ADDRESS};
use steel::*;

use crate::{
    error::ApiError,
    instruction::*,
    state::{
        member_pda, pool_pda, pool_proof_pda, pool_share_rewards_pda, pool_total_rewards, share_pda,
    },
};

/// Builds a launch instruction.
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
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(beneficiary, false),
            AccountMeta::new(member_pda, false),
            AccountMeta::new(pool_pda, false),
            AccountMeta::new(pool_proof_pda, false),
            AccountMeta::new_readonly(TREASURY_ADDRESS, false),
            AccountMeta::new(TREASURY_TOKENS_ADDRESS, false),
            AccountMeta::new_readonly(ore_api::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
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
        program_id: crate::ID,
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

/// Builds a commit instruction.
pub fn commit(signer: Pubkey, mint: Pubkey) -> Instruction {
    let (boost_pda, _) = ore_boost_api::state::boost_pda(mint);
    let boost_tokens =
        spl_associated_token_account::get_associated_token_address(&boost_pda, &mint);
    let (pool_pda, _) = pool_pda(signer);
    let pool_tokens = spl_associated_token_account::get_associated_token_address(&pool_pda, &mint);
    let (stake_pda, _) = ore_boost_api::state::stake_pda(pool_pda, boost_pda);
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(boost_pda, false),
            AccountMeta::new(boost_tokens, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(pool_pda, false),
            AccountMeta::new(pool_tokens, false),
            AccountMeta::new(stake_pda, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(ore_boost_api::ID, false),
        ],
        data: Commit {}.to_bytes(),
    }
}

/// Builds an submit instruction.
pub fn submit(
    signer: Pubkey,
    solution: Solution,
    attestation: [u8; 32],
    bus: Pubkey,
    boost_accounts: Vec<Pubkey>,
) -> Instruction {
    let (pool_pda, _) = pool_pda(signer);
    let (proof_pda, _) = pool_proof_pda(pool_pda);
    let accounts = vec![
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
    let boost_accounts = boost_accounts
        .into_iter()
        .map(|pk| AccountMeta::new_readonly(pk, false))
        .collect();
    let accounts = [accounts, boost_accounts].concat();
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

/// builds an unstake instruction.
pub fn unstake(
    signer: Pubkey,
    mint: Pubkey,
    pool: Pubkey,
    recipient: Pubkey,
    amount: u64,
) -> Instruction {
    let (boost_pda, _) = ore_boost_api::state::boost_pda(mint);
    let boost_tokens =
        spl_associated_token_account::get_associated_token_address(&boost_pda, &mint);
    let (member_pda, _) = member_pda(signer, pool);
    let pool_tokens = spl_associated_token_account::get_associated_token_address(&pool, &mint);
    let (share_pda, _) = share_pda(signer, pool, mint);
    let (stake_pda, _) = ore_boost_api::state::stake_pda(pool, boost_pda);
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
            AccountMeta::new_readonly(ore_boost_api::ID, false),
        ],
        data: Unstake {
            amount: amount.to_le_bytes(),
        }
        .to_bytes(),
    }
}

/// builds a stake instruction.
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

/// Builds an open share rewards instruction.
pub fn open_share_rewards(signer: Pubkey, pool: Pubkey, mint: Pubkey) -> Instruction {
    let (share_rewards_pda, bump) = pool_share_rewards_pda(pool, mint);
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(pool, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(share_rewards_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: OpenShareRewards { bump }.to_bytes(),
    }
}

/// Builds an open total rewards instruction.
pub fn open_total_rewards(signer: Pubkey, pool: Pubkey) -> Instruction {
    let (total_rewards_pda, bump) = pool_total_rewards(pool);
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(pool, false),
            AccountMeta::new(total_rewards_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: OpenTotalRewards { bump }.to_bytes(),
    }
}

/// Builds an open share instruction.
pub fn open_share(signer: Pubkey, mint: Pubkey, pool: Pubkey) -> Instruction {
    let (boost_pda, _) = ore_boost_api::state::boost_pda(mint);
    let (share_pda, share_bump) = share_pda(signer, pool, mint);
    let (stake_pda, _) = ore_boost_api::state::stake_pda(pool, boost_pda);
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(boost_pda, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(pool, false),
            AccountMeta::new(share_pda, false),
            AccountMeta::new_readonly(stake_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: OpenShare { share_bump }.to_bytes(),
    }
}

/// Builds an open stake instruction.
pub fn open_stake(signer: Pubkey, mint: Pubkey) -> Instruction {
    let (boost_pda, _) = ore_boost_api::state::boost_pda(mint);
    let (pool_pda, _) = pool_pda(signer);
    let pool_tokens = spl_associated_token_account::get_associated_token_address(&pool_pda, &mint);
    let (stake_pda, _) = ore_boost_api::state::stake_pda(pool_pda, boost_pda);
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(boost_pda, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(pool_pda, false),
            AccountMeta::new(pool_tokens, false),
            AccountMeta::new(stake_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
            AccountMeta::new_readonly(ore_boost_api::ID, false),
        ],
        data: OpenStake {}.to_bytes(),
    }
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
