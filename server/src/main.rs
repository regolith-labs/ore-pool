mod contribute;
mod database;
mod utils;

use std::{collections::HashMap, sync::Arc};

use actix_web::{middleware, web, App, HttpServer};
use contribute::*;
use database::create_pool;
use sha3::{Digest, Sha3_256};
use solana_sdk::{compute_budget::ComputeBudgetInstruction, signature::Keypair, signer::Signer};
use tokio::sync::RwLock;
use utils::{create_cors, rpc_client, Aggregator, Challenge, Contribution};

// TODO Register endpoint

// TODO Start endpoint
// TODO Fetch current challenge, schedule timer

// TODO Timer loop to kickoff submission jobs

// TODO Timer loop to attribute on-chain balances
// TODO Make this idempotent to avoid duplication

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let pool = create_pool();
    let mut aggregator = Aggregator::default();
    let challenge = Arc::new(RwLock::new(Challenge::default()));
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Contribution>(1000);

    // Aggregate contributions
    tokio::task::spawn(async move {
        while let Some(contribution) = rx.recv().await {
            let difficulty = contribution.solution.to_hash().difficulty();
            let score = 2u64.pow(difficulty);
            aggregator.total_score += score;
            aggregator
                .contributions
                .insert(contribution.member, contribution);

            // TODO If time, kick off submission
        }
    });

    // Kick off attribution loop
    tokio::task::spawn(async move {
        // TODO Every 20 minutes update user's on-chain claimable balances
    });

    // Launch server
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(challenge.clone()))
            .app_data(web::Data::new(rpc_client()))
            .app_data(web::Data::new(tx.clone()))
            .service(
                web::resource("/contribute")
                    .wrap(create_cors())
                    .route(web::post().to(contribute)),
            )
    })
    .bind("0.0.0.0:3000")?
    .run()
    .await
}

pub async fn submit(aggregator: &mut Aggregator, challenge: Arc<RwLock<Challenge>>) {
    // Get the current batch.
    let mut contributions: Vec<&Contribution> = aggregator.contributions.values().collect();
    contributions.sort_by(|a, b| a.score.cmp(&b.score));

    // Get the best hash
    let best_solution = contributions[0].solution;

    // Generate attestation
    let mut hasher = Sha3_256::new();
    let mut block = String::new();
    for contribution in contributions {
        let hex_string: String = contribution
            .solution
            .d
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect();
        let line = format!(
            "{} {} {}\n",
            contribution.member.to_string(),
            hex_string,
            u64::from_le_bytes(contribution.solution.n)
        );
        block.push_str(&line);
        hasher.update(&line);
    }

    // Generate attestation
    let mut attestation: [u8; 32] = [0; 32];
    attestation.copy_from_slice(&hasher.finalize()[..]);

    // TODO Submit best hash and attestation to Solana
    let keypair = Keypair::new(); // TODO
    let rpc = rpc_client();
    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(600_000);
    let compute_price_ix = ComputeBudgetInstruction::set_compute_unit_price(100_000); // TODO
    let ix = ore_pool_api::instruction::submit(keypair.pubkey(), best_solution, attestation);

    // TODO Parse tx response
    // TODO Update members' local attribution balances
    // TODO Publish block to S3
    // TODO Refresh the challenge

    // Reset the aggregator
    aggregator.contributions = HashMap::new();
    aggregator.total_score = 0;
}
