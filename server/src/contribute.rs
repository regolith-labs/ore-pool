use actix_web::{web, HttpResponse, Responder};
use drillx::Solution;
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

use crate::{aggregator::Aggregator, Contribution};

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

/// Accepts solutions from pool members. If their solutions are valid, it
/// aggregates the contributions into a list for publishing and submission.
pub async fn contribute(
    payload: web::Json<ContributePayload>,
    tx: web::Data<tokio::sync::mpsc::Sender<Contribution>>,
    aggregator: web::Data<Aggregator>,
) -> impl Responder {
    let aggregator = aggregator.as_ref();
    // Authenticate the sender signature
    if !payload
        .signature
        .verify(&payload.authority.to_bytes(), &payload.solution.to_bytes())
    {
        return HttpResponse::Unauthorized().finish();
    }

    // TODO: Validate sender is an accepted member of the pool

    // Return error if digest is invalid
    if !drillx::is_valid_digest(
        &aggregator.challenge.challenge,
        &payload.solution.n,
        &payload.solution.d,
    ) {
        return HttpResponse::BadRequest().finish();
    }

    // Calculate score
    let difficulty = payload.solution.to_hash().difficulty();
    let score = 2u64.pow(difficulty);

    // TODO: Reject if score is below min difficulty

    // Update the aggegator
    tx.send(Contribution {
        member: payload.authority,
        score,
        solution: payload.solution,
    })
    .await
    .ok();

    HttpResponse::Ok().finish()
}
