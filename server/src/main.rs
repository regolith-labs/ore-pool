mod database;

use std::{collections::HashMap, sync::Arc};

use actix_cors::Cors;
use actix_web::{http::header, middleware, web, App, HttpResponse, HttpServer, Responder};
use database::create_pool;
use drillx::Solution;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use tokio::sync::{Mutex, RwLock};

// TODO Persistent database to hold onto pool participant balances

// TODO Register endpoint
// TODO KYC process?

// TODO Start endpoint
// TODO Fetch challenge,

// TODO Endpoint to receive hashes
// TODO Validate sender is a pool participant

// TODO Timer to kickoff submission jobs
// TODO Batch all hashes into a file
// TODO Hash the file to generate the attestation
// TODO Submit the best hash and attestation to Solana
// TODO Publish the file to S3

// TODO Every 10 minutes, update user's on-chain balances
// TODO Make this idempotent to avoid duplication

pub struct Hash {
    pub solution: Solution,
    pub score: u64,
}

#[derive(Default)]
pub struct Challenge {
    pub challenge: [u8; 32],
}

#[derive(Default)]
pub struct Aggregator {
    pub hashes: HashMap<Pubkey, Hash>,
    pub total_score: u64,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let pool = create_pool();
    let aggregator = Arc::new(Mutex::new(Aggregator::default()));
    let challenge = Arc::new(RwLock::new(Aggregator::default()));

    // TODO Start by fetching the current challenge
    // TODO Kick off submission loop

    // Launch server
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(aggregator.clone()))
            .app_data(web::Data::new(challenge.clone()))
            .service(
                web::resource("/submit")
                    .wrap(create_cors())
                    .route(web::post().to(submit)),
            )
    })
    .bind("0.0.0.0:3000")?
    .run()
    .await
}

fn create_cors() -> Cors {
    Cors::default()
        .allowed_origin_fn(|_origin, _req_head| {
            // origin.as_bytes().ends_with(b"ore.supply") || // Production origin
            // origin == "http://localhost:8080" // Local development origin
            true
        })
        .allowed_methods(vec!["GET", "POST"]) // Methods you want to allow
        .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
        .allowed_header(header::CONTENT_TYPE)
        .max_age(3600)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SubmitPayload {
    pub authority: Pubkey,
    pub solution: Solution,
    pub sig: [u8; 32],
}

pub async fn submit(
    payload: web::Json<SubmitPayload>,
    aggregator: web::Data<Arc<Mutex<Aggregator>>>,
    challenge: web::Data<Arc<RwLock<Challenge>>>,
) -> impl Responder {
    // TODO Authenticate the sender via signature
    // TODO Validate member is approved and accepted into the pool

    // Return error if digest is invalid
    let challenge = challenge.read().await.challenge;
    if !drillx::is_valid_digest(&challenge, &payload.solution.n, &payload.solution.d) {
        return HttpResponse::Ok();
    }

    // Calculate score
    let difficulty = payload.solution.to_hash().difficulty();
    let score = 2u64.pow(difficulty);

    // TODO Reject if below min difficulty

    // Update the aggegator
    let mut w_aggregator = aggregator.lock().await;
    w_aggregator.total_score += score;
    w_aggregator.hashes.insert(
        payload.authority,
        Hash {
            solution: payload.solution,
            score,
        },
    );
    drop(w_aggregator);

    HttpResponse::Ok()
}
