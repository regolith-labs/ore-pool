mod contribute;
mod database;
mod utils;

use std::{collections::HashMap, sync::Arc};

use actix_web::{middleware, web, App, HttpServer};
use contribute::*;
use database::create_pool;
use drillx::Solution;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::{Mutex, RwLock};
use utils::{create_cors, rpc_client};

// TODO Persistent database to hold onto pool participant balances

// TODO Register endpoint
// TODO KYC process? This should just be a flag in the database.

// TODO Start endpoint
// TODO Fetch current challenge, schedule timer

// TODO Timer to kickoff submission jobs
// TODO Batch all hashes into a file
// TODO Hash the file to generate the attestation
// TODO Submit the best hash and attestation to Solana
// TODO Publish the file to S3

// TODO Every 10 minutes, update user's on-chain balances
// TODO Make this idempotent to avoid duplication

/// The current challenge the pool is accepting solutions for.
type Challenge = [u8; 32];

/// Aggregates contributions from the pool members.
#[derive(Default)]
pub struct Aggregator {
    /// The set of contributions aggregated for the current challenge.
    pub contributions: HashMap<Pubkey, Contribution>,

    /// The total difficulty score of all the contributions aggregated so far.
    pub total_score: u64,
}

/// A recorded contribution from a particular member of the pool.
pub struct Contribution {
    /// The drillx solution submitted representing the member's best hash.
    pub solution: Solution,

    /// The difficulty score of the solution.
    pub score: u64,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let pool = create_pool();
    let aggregator = Arc::new(Mutex::new(Aggregator::default()));
    let challenge = Arc::new(RwLock::new(Challenge::default()));

    // TODO Start by fetching the current challenge
    // TODO Kick off submission loop

    // Launch server
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(aggregator.clone()))
            .app_data(web::Data::new(challenge.clone()))
            .app_data(web::Data::new(rpc_client()))
            .service(
                web::resource("/submit")
                    .wrap(create_cors())
                    .route(web::post().to(contribute)),
            )
    })
    .bind("0.0.0.0:3000")?
    .run()
    .await
}

pub async fn submit(aggregator: Arc<Mutex<Aggregator>>, challenge: Arc<RwLock<Challenge>>) {
    // Get the current batch.
    let mut aggregator = aggregator.lock().await;
    let mut x: Vec<&Contribution> = aggregator.contributions.values().collect();
    x.sort_by(|a, b| a.score.cmp(&b.score));

    // TODO Generate attestation
    // TODO Submit best hash to Solana
    let rpc = rpc_client();

    // TODO Write all contributions to file
    // TODO Publish file to S3

    // TODO Refresh the challenge

    // Reset the aggregator
    aggregator.contributions = HashMap::new();
    aggregator.total_score = 0;
}
