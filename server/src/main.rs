mod aggregator;
mod handlers;
mod database;
mod error;
mod contributions;
mod operator;
mod tx;
mod utils;
mod webhook;

use core::panic;

use actix_web::{get, middleware, web, App, HttpResponse, HttpServer, Responder};
use aggregator::Aggregator;
use contributions::{Contribution, PoolMiningEvent};
use operator::Operator;
use utils::create_cors;

// TODO: publish attestation to s3
// write attestation url to db with last-hash-at as foreign key
#[actix_web::main]
async fn main() -> Result<(), error::Error> {
    env_logger::init();
    // events channel
    let (events_tx, mut events_rx) = tokio::sync::mpsc::channel::<PoolMiningEvent>(1);
    let events_tx = web::Data::new(events_tx);

    // contributions channel
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Contribution>();
    let tx = web::Data::new(tx);

    // operator and aggregator mutex
    let operator = web::Data::new(Operator::new()?);
    let aggregator = web::Data::new(tokio::sync::RwLock::new(Aggregator::new(&operator).await?));

    // env vars
    let attribution_epoch = attribution_epoch()?;

    // aggregate contributions
    tokio::task::spawn({
        let operator = operator.clone();
        let aggregator = aggregator.clone();
        async move {
            if let Err(err) =
                aggregator::process_contributions(aggregator.as_ref(), operator.as_ref(), &mut rx)
                    .await
            {
                log::error!("{:?}", err);
            }
        }
    });

    // distribute rewards
    tokio::task::spawn({
        let operator = operator.clone();
        let aggregator = aggregator.clone();
        async move {
            loop {
                match events_rx.recv().await {
                    Some(rewards) => {
                        let mut aggregator = aggregator.write().await;
                        if let Err(err) = aggregator
                            .distribute_rewards(operator.as_ref(), &rewards)
                            .await
                        {
                            log::error!("{:?}", err);
                        }
                    }
                    None => {
                        panic!("rewards channel closed")
                    }
                };
            }
        }
    });

    // kick off attribution loop
    tokio::task::spawn({
        let operator = operator.clone();
        async move {
            loop {
                // submit attributions
                let operator = operator.clone().into_inner();
                if let Err(err) = operator.attribute_members().await {
                    panic!("{:?}", err)
                }
                // sleep until next epoch
                tokio::time::sleep(tokio::time::Duration::from_secs(60 * attribution_epoch)).await;
            }
        }
    });

    // launch server
    HttpServer::new(move || {
        log::info!("starting pool server");
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(create_cors())
            .app_data(tx.clone())
            .app_data(operator.clone())
            .app_data(aggregator.clone())
            .app_data(events_tx.clone())
            .service(web::resource("/address").route(web::get().to(handlers::address)))
            .service(web::resource("/challenge").route(web::get().to(handlers::challenge)))
            .service(web::resource("/challenge/{authority}").route(web::get().to(handlers::challenge)))
            .service(web::resource("/contribute").route(web::post().to(handlers::contribute)))
            .service(web::resource("/commit").route(web::post().to(handlers::commit_balance)))
            .service(web::resource("/event/latest/{authority}").route(web::get().to(handlers::latest_event)))
            .service(web::resource("/member/{authority}").route(web::get().to(handlers::member)))
            .service(web::resource("/register").route(web::post().to(handlers::register)))
            .service(web::resource("/webhook/rewards").route(web::post().to(webhook::mine_event)))
            .service(health)
    })
    .bind("0.0.0.0:3000")?
    .run()
    .await
    .map_err(From::from)
}

// denominated in minutes
fn attribution_epoch() -> Result<u64, error::Error> {
    let string = std::env::var("ATTR_EPOCH")?;
    let epoch: u64 = string.parse()?;
    Ok(epoch)
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().body("ok")
}
