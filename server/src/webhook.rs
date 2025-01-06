use std::collections::HashMap;

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use base64::{prelude::BASE64_STANDARD, Engine};
use cached::proc_macro::cached;
use solana_sdk::signature::Signature;

use crate::{contributions::PoolMiningEvent, error::Error};

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RawPayload {
    pub block_time: u64,
    pub slot: u64,
    pub meta: Meta,
    pub signatures: Vec<Signature>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub log_messages: Vec<String>,
}

pub async fn mine_event(
    tx: web::Data<tokio::sync::mpsc::Sender<PoolMiningEvent>>,
    req: HttpRequest,
    bytes: web::Bytes,
) -> impl Responder {
    // Validate auth header
    if let Err(err) = auth(&req) {
        log::error!("{:?}", err);
        return HttpResponse::Unauthorized().finish();
    }

    // Parse payload
    let bytes = bytes.to_vec();
    let json = match serde_json::from_slice::<serde_json::Value>(bytes.as_slice()) {
        Ok(json) => json,
        Err(err) => {
            log::error!("{:?}", err);
            return HttpResponse::BadRequest().finish();
        }
    };
    let payload = match serde_json::from_value::<Vec<RawPayload>>(json) {
        Ok(payload) => payload,
        Err(err) => {
            log::error!("{:?}", err);
            return HttpResponse::BadRequest().finish();
        }
    };
    let payload = payload.first().unwrap();

    // Parse mine event from transaction logs
    let mine_event = match parse_mine_event(&payload) {
        Ok(event) => event,
        Err(err) => {
            log::error!("{:?}", err);
            return HttpResponse::BadRequest().finish();
        }
    };

    // Submit mine event to aggregator
    let event = PoolMiningEvent {
        signature: payload.signatures.first().unwrap().clone(),
        block: payload.slot,
        timestamp: payload.block_time,
        mine_event: mine_event.clone(),
        member_rewards: HashMap::new(),
        member_scores: HashMap::new(),
    };
    if let Err(err) = tx.send(event).await {
        log::error!("{:?}", err);
        return HttpResponse::InternalServerError().finish();
    }

    // Return success
    HttpResponse::Ok().finish()
}


/// Parse a MineEvent from a Helius webhook event
fn parse_mine_event(
    payload: &RawPayload,
) -> Result<ore_api::event::MineEvent, Error> {
    // Parse the payload
    let log_messages = payload.meta.log_messages.as_slice();
    let index = log_messages.len().checked_sub(2).ok_or(Error::Internal(
        "invalid webhook event message index".to_string(),
    ))?;

    // Parse the mine event
    let mine_event = log_messages
        .get(index)
        .ok_or(Error::Internal("missing webhook base reward".to_string()))?;
    let mine_event = mine_event
    .trim_start_matches(format!("Program return: {} ", ore_pool_api::ID).as_str());
    let mine_event = BASE64_STANDARD.decode(mine_event)?;
    let mine_event: &ore_api::event::MineEvent =
        bytemuck::try_from_bytes(mine_event.as_slice())
            .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(*mine_event)
}

/// Validate the auth header
fn auth(req: &HttpRequest) -> Result<(), Error> {
    let header = req.headers().get("Authorization").ok_or(Error::Internal(
        "missing auth header in webhook event".to_string(),
    ))?;
    let header = header.to_str()?;
    if header.to_string().ne(&helius_auth_token()) {
        return Err(Error::Internal(
            "invalid auth header in webhook event".to_string(),
        ));
    }
    Ok(())
}

#[cached]
fn helius_auth_token() -> String {
    std::env::var("HELIUS_AUTH_TOKEN").expect("No helius auth token found")
}
