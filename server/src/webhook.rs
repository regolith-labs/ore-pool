use actix_web::{web, HttpRequest, HttpResponse, Responder};
use base64::{prelude::BASE64_STANDARD, Engine};
use cached::proc_macro::cached;

use crate::error::Error;

#[derive(serde::Deserialize, Debug)]
pub struct RawPayload {
    pub meta: Meta,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub log_messages: Vec<String>,
}

pub async fn mine_event(
    tx: web::Data<tokio::sync::mpsc::Sender<ore_api::event::MineEvent>>,
    req: HttpRequest,
    bytes: web::Bytes,
) -> impl Responder {
    // Validate auth header
    if let Err(err) = auth(&req) {
        log::error!("{:?}", err);
        return HttpResponse::Unauthorized().finish();
    }

    // Parse mine event from transaction logs
    let mine_event = match parse_mine_event(&req, &bytes) {
        Ok(event) => event,
        Err(err) => {
            log::error!("{:?}", err);
            return HttpResponse::BadRequest().finish();
        }
    };

    // Submit mine event to aggregator
    if let Err(err) = tx.send(mine_event).await {
        log::error!("{:?}", err);
        return HttpResponse::InternalServerError().finish();
    }

    // Return success
    HttpResponse::Ok().finish()
}

/// Parse a MineEvent from a Helius webhook event
fn parse_mine_event(
    req: &HttpRequest,
    bytes: &web::Bytes,
) -> Result<ore_api::event::MineEvent, Error> {
    // Authorize request
    auth(req)?;

    // Decode payload
    let bytes = bytes.to_vec();
    let json = serde_json::from_slice::<serde_json::Value>(bytes.as_slice())?;
    let payload = serde_json::from_value::<Vec<RawPayload>>(json)?;

    // Parse the payload
    let payload = payload
        .first()
        .ok_or(Error::Internal("empty webhook event".to_string()))?;
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
