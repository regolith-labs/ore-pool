use drillx::Solution;
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

#[derive(Debug, Deserialize)]
pub struct GetChallengePayload {
    /// The authority of the member account sending the payload.
    pub authority: Pubkey,
}

// The response from the /challenge request.
#[derive(Debug, Serialize)]
pub struct MemberChallenge {
    // The challenge to mine for.
    pub challenge: [u8; 32],

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
