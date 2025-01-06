use drillx::Solution;
use serde::{Deserialize, Serialize};
use solana_sdk::{hash::Hash, pubkey::Pubkey, signature::Signature, transaction::Transaction};

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterPayload {
    /// The authority of the member account sending the payload.
    pub authority: Pubkey,
}

#[derive(Debug, Deserialize)]
pub struct GetMemberPayload {
    /// The authority of the member account sending the payload.
    pub authority: String,
}

#[derive(Debug, Deserialize)]
pub struct GetEventPayload {
    /// The authority of the member account sending the payload.
    pub authority: String,
}


#[derive(Debug, Deserialize)]
pub struct GetChallengePayload {
    /// The authority of the member account sending the payload.
    pub authority: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContributePayload {
    /// The authority of the member account sending the payload.
    pub authority: Pubkey,

    /// The solution submitted.
    pub solution: Solution,

    /// Must be a valid signature of the solution
    pub signature: Signature,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContributePayloadV2 {
    /// The authority of the member account sending the payload.
    pub authority: Pubkey,

    /// The solution submitted.
    pub solution: Solution,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateBalancePayload {
    /// The authority of the member account sending the payload.
    pub authority: Pubkey,

    /// The transaction containing the attribute instruction
    /// signed by the client as fee payer.
    pub transaction: Transaction,

    /// The hash used to signed the transaction on the client.
    pub hash: Hash,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterStakerPayload {
    /// The authority of the member account sending the payload.
    pub authority: Pubkey,

    /// The mint for the boost account the member is staking to.
    pub mint: Pubkey,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PoolAddress {
    /// The pubkey address of the pool pda of this operator.
    pub address: Pubkey,

    /// The bump returned when deriving the pda.
    pub bump: u8,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Challenge {
    /// The current challenge the pool is accepting solutions for.
    pub challenge: [u8; 32],

    /// Foreign key to the ORE proof account.
    pub lash_hash_at: i64,

    // The current minimum difficulty accepted by the ORE program.
    pub min_difficulty: u64,

    // The cutoff time to stop accepting contributions.
    pub cutoff_time: u64,
}

/// The member record that sits in the operator database
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Member {
    /// The respective pda pubkey of the on-chain account.
    pub address: String,

    /// The id as assigned by the on-chain program.
    pub id: i64,

    /// The authority pubkey of this account.
    pub authority: String,

    /// The pool pubkey this account belongs to.
    pub pool_address: String,

    /// The total balance assigned to this account.
    /// Always increments for idempotent on-chain updates.
    pub total_balance: i64,

    /// Whether or not this member is approved by the operator.
    pub is_approved: bool,

    /// Whether or not this member is KYC'd by the operator.
    pub is_kyc: bool,

    /// Whether or not this member's on-chain balance is in sync with the operator db balance.
    pub is_synced: bool,
}

/// The staker record that sits in the operator database
#[derive(Debug, Serialize, Deserialize)]
pub struct Staker {
    /// the share account address
    pub address: Pubkey,

    /// the member id (foreign key relation to members table)
    pub member_id: u64,

    /// the mint of the boost account the member is staking to
    pub mint: Pubkey,

    /// whether or not this account has been added to the webhook
    pub webhook: bool,
}

/// The response from the /challenge request.
#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct MemberChallenge {
    /// The challenge to mine for.
    pub challenge: Challenge,

    /// The number of total members to divide the nonce space by.
    pub num_total_members: u64,

    /// The id/index for distinguishing devices the client is using.
    pub device_id: u8,

    /// The number of client devices permitted per member.
    pub num_devices: u8,
}

/// The response from the update-balance request.
#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceUpdate {
    /// The balance updated on-chain.
    pub balance: u64,

    /// The transaction signature.
    pub signature: Signature,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PoolMemberMiningEvent {
    pub signature: Signature,
    pub block: u64,
    pub timestamp: u64,
    pub balance: u64,
    pub difficulty: u64,
    pub last_hash_at: i64,
    pub timing: i64,
    pub net_reward: u64,
    pub net_base_reward: u64,
    pub net_miner_boost_reward: u64,
    pub net_staker_boost_reward: u64,
    pub member_difficulty: u64,
    pub member_reward: u64
}
