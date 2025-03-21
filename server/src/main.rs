mod aggregator;
mod contributions;
mod database;
mod error;
mod handlers;
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
    // env vars
    let attribution_epoch = attribution_epoch()?;

    // events channel
    let (events_tx, mut events_rx) = tokio::sync::mpsc::channel::<PoolMiningEvent>(1);
    let events_tx = web::Data::new(events_tx);

    // contributions channel
    let (contributions_tx, mut contributions_rx) =
        tokio::sync::mpsc::unbounded_channel::<Contribution>();
    let contributions_tx = web::Data::new(contributions_tx);

    // clock channel
    let (clock_tx, _) = tokio::sync::broadcast::channel::<i64>(1);
    let clock_tx = web::Data::new(clock_tx);

    // operator and aggregator mutex
    let operator = web::Data::new(Operator::new()?);
    let aggregator = web::Data::new(tokio::sync::RwLock::new(Aggregator::new(&operator).await?));

    // aggregate contributions
    tokio::task::spawn({
        let operator = operator.clone();
        let aggregator = aggregator.clone();
        async move {
            if let Err(err) = aggregator::process_contributions(
                aggregator.as_ref(),
                operator.as_ref(),
                &mut contributions_rx,
            )
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

    // clock
    tokio::task::spawn({
        let operator = operator.clone();
        let clock_tx = clock_tx.clone();
        async move {
            // every 10 seconds fetch the rpc clock
            loop {
                let mut ticks = 0;
                let mut unix_timestamp = match operator.get_clock().await {
                    Ok(clock) => clock.unix_timestamp,
                    Err(err) => {
                        log::error!("{:?}", err);
                        continue;
                    }
                };
                // every 1 seconds simulate a tick
                loop {
                    // reset every 10 ticks
                    ticks += 1;
                    if ticks.eq(&10) {
                        break;
                    }
                    // send tick
                    let _ = clock_tx.send(unix_timestamp);
                    // simulate tick of rpc clock
                    unix_timestamp += 1;
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    });

    // launch server
    HttpServer::new(move || {
        log::info!("starting pool server");
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(create_cors())
            .app_data(contributions_tx.clone())
            .app_data(clock_tx.clone())
            .app_data(operator.clone())
            .app_data(aggregator.clone())
            .app_data(events_tx.clone())
            .service(web::resource("/address").route(web::get().to(handlers::address)))
            .service(web::resource("/challenge").route(web::get().to(handlers::challenge)))
            .service(
                web::resource("/challenge/{authority}").route(web::get().to(handlers::challenge)),
            )
            .service(web::resource("/contribute").route(web::post().to(handlers::contribute)))
            .service(web::resource("/commit").route(web::post().to(handlers::commit_balance)))
            .service(
                web::resource("/event/latest/{authority}")
                    .route(web::get().to(handlers::latest_event)),
            )
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
