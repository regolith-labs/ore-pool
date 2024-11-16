mod aggregator;
mod contributor;
mod database;
mod error;
mod miner;
mod operator;
mod staker;
mod tx;
mod utils;
mod webhook;

use core::panic;
use std::{collections::HashMap, sync::Arc};

use actix_web::{get, middleware, web, App, HttpResponse, HttpServer, Responder};
use aggregator::Aggregator;
use miner::Contribution;
use operator::Operator;
use staker::Stakers;
use utils::create_cors;

// TODO: publish attestation to s3
// write attestation url to db with last-hash-at as foreign key
#[actix_web::main]
async fn main() -> Result<(), error::Error> {
    env_logger::init();
    // rewards channel
    let (rewards_tx, mut rewards_rx) =
        tokio::sync::mpsc::channel::<(ore_api::event::MineEvent, webhook::BoostAccounts)>(1);
    let rewards_tx = web::Data::new(rewards_tx);
    // contributions channel
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Contribution>();
    let tx = web::Data::new(tx);
    // operator and aggregator mutex
    let operator = web::Data::new(Operator::new()?);
    let aggregator = tokio::sync::RwLock::new(Aggregator::new(&operator).await?);
    let aggregator = web::Data::new(aggregator);
    let webhook_handler = web::Data::new(webhook::Handle::new()?);
    let webhook_client = web::Data::new(webhook::Client::new_stake()?);
    // env vars
    let attribution_epoch = attribution_epoch()?;
    let stake_commit_epoch = stake_commit_epoch()?;

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
                match rewards_rx.recv().await {
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

    // kick off commit-stake loop
    tokio::task::spawn({
        let operator = operator.clone();
        let aggregator = aggregator.clone();
        async move {
            loop {
                let operator = operator.clone().into_inner();
                let aggregator = aggregator.clone().into_inner();
                // commit stake
                if let Err(err) = commit_stake(operator, aggregator).await {
                    log::error!("{:?}", err);
                }
                // sleep until next epoch
                tokio::time::sleep(tokio::time::Duration::from_secs(60 * stake_commit_epoch)).await;
            }
        }
    });

    // launch server
    HttpServer::new(move || {
        log::info!("starting server");
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(create_cors())
            .app_data(tx.clone())
            .app_data(operator.clone())
            .app_data(aggregator.clone())
            .app_data(webhook_handler.clone())
            .app_data(webhook_client.clone())
            .app_data(rewards_tx.clone())
            .service(web::resource("/member/{authority}").route(web::get().to(contributor::member)))
            .service(web::resource("/pool-address").route(web::get().to(contributor::pool_address)))
            .service(web::resource("/register").route(web::post().to(contributor::register)))
            .service(
                web::resource("/register-staker")
                    .route(web::post().to(contributor::register_staker)),
            )
            .service(web::resource("/contribute").route(web::post().to(contributor::contribute)))
            .service(web::resource("/challenge").route(web::get().to(contributor::challenge)))
            .service(
                web::resource("/update-balance").route(web::post().to(contributor::update_balance)),
            )
            .service(
                web::resource("/webhook/share-account")
                    .route(web::post().to(webhook::Handle::share_account)),
            )
            .service(
                web::resource("/webhook/rewards").route(web::post().to(webhook::Handle::rewards)),
            )
            .service(health)
    })
    .bind("0.0.0.0:3000")?
    .run()
    .await
    .map_err(From::from)
}

async fn commit_stake(
    operator: Arc<Operator>,
    aggregator: Arc<tokio::sync::RwLock<Aggregator>>,
) -> Result<(), error::Error> {
    // commit stake
    operator.commit_stake().await?;
    // lock aggregator
    let aggregator = &mut aggregator.write().await;
    // update staker balances
    let mut stake: Stakers = HashMap::new();
    let boost_accounts = operator.boost_accounts.iter();
    for ba in boost_accounts {
        let stakers = operator.get_stakers_onchain(&ba.mint).await?;
        stake.insert(ba.mint, stakers);
    }
    // set stakers
    aggregator.stake = stake;
    Ok(())
}

// denominated in minutes
fn stake_commit_epoch() -> Result<u64, error::Error> {
    let string = std::env::var("STAKE_EPOCH")?;
    let epoch: u64 = string.parse()?;
    Ok(epoch)
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
