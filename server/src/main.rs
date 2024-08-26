mod aggregator;
mod contribute;
mod database;
mod error;
mod operator;
mod utils;

use actix_web::{middleware, web, App, HttpServer};
use aggregator::{Aggregator, Contribution};
use contribute::*;
use database::create_pool;
use operator::Operator;
use utils::create_cors;

// TODO Register endpoint

// TODO Start endpoint
// TODO Fetch current challenge, schedule timer

// TODO Timer loop to kickoff submission jobs

// TODO Timer loop to attribute on-chain balances
// TODO Make this idempotent to avoid duplication

#[actix_web::main]
async fn main() -> Result<(), error::Error> {
    env_logger::init();
    let pool = create_pool();
    let operator = web::Data::new(Operator::new()?);
    let aggregator = web::Data::new(Aggregator::new(&operator).await?);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Contribution>();

    // Aggregate contributions
    tokio::task::spawn(async move {
        // while let Some(contribution) = rx.recv().await {
        //     aggregator.insert(&contribution, winner)
        //     // TODO If time, kick off submission
        // }
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
            .app_data(web::Data::new(tx.clone()))
            .app_data(operator.clone())
            .app_data(aggregator.clone())
            .service(
                web::resource("/contribute")
                    .wrap(create_cors())
                    .route(web::post().to(contribute)),
            )
    })
    .bind("0.0.0.0:3000")?
    .run()
    .await
    .map_err(From::from)
}
