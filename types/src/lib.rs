use drillx::Solution;
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterPayload {
    /// The authority of the member account sending the payload.
    pub authority: Pubkey,
}

#[derive(Debug, Deserialize)]
pub struct GetChallengePayload {
    /// The authority of the member account sending the payload.
    pub authority: String,
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

// The member record that sits in the operator database
#[derive(Debug, Serialize, Deserialize)]
pub struct Member {
    // The respective pda pubkey of the on-chain account.
    pub address: String,

    // The id as assigned by the on-chain program.
    pub id: i64,

    // The authority pubkey of this account.
    pub authority: String,

    // The pool pubkey this account belongs to.
    pub pool_address: String,

    // The total balance assigned to this account.
    // Always increments for idempotent on-chain updates.
    pub total_balance: i64,

    // Whether or not this member is approved by the operator.
    pub is_approved: bool,

    // Whether or not this member is KYC'd by the operator.
    pub is_kyc: bool,
}

// The response from the /challenge request.
#[derive(Debug, Serialize, Deserialize)]
pub struct MemberChallenge {
    // The challenge to mine for.
    pub challenge: Challenge,

    // The unique nonce index to start mining at.
    pub nonce_index: u64,

    // The number of total members to divide the nonce space by.
    pub num_total_members: u64,
}

/// The payload to send to the /contribute endpoint.
#[derive(Debug, Deserialize, Serialize)]
pub struct ContributePayload {
    /// The authority of the member account sending the payload.
    pub authority: Pubkey,

    /// The solution submitted.
    pub solution: Solution,

    /// Must be a valid signature of the solution
    pub signature: Signature,
}
