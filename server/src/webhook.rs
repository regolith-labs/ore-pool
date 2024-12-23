use actix_web::{web, HttpRequest, HttpResponse, Responder};
use base64::{prelude::BASE64_STANDARD, Engine};

use crate::error::Error;

const HELIUS_URL: &str = "https://api.helius.xyz";
const HELIUS_WEBHOOK_API_PATH: &str = "v0/webhooks";
const HELIUS_WEBHOOK_TYPE: &str = "raw";
const HELIUS_TRANSACTION_TYPE: &str = "all";

/// client for managing helius webhooks
pub struct Client {
    http_client: reqwest::Client,
    /// query paramter added to the url for making http requets to helius
    helius_api_key: String,
    /// the helius webhook id created in the console
    /// for tracking share accounts
    helius_webhook_id: String,
    /// the /webhook path that your server exposes to helius
    helius_webhook_url: String,
    /// the auth token expected to be included in webhook events
    /// posted from helius to our server.
    helius_auth_token: String,
}

/// handler for receiving helius webhook events
pub struct Handle {
    /// the auth token expected to be included in webhook events
    /// posted from helius to our server.
    helius_auth_token: String,
}


#[derive(serde::Deserialize, Debug)]
struct ClientEditSuccess {
    #[serde(rename = "webhookID")]
    webhook_id: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct Event {
    pub meta: EventMeta,
    pub transaction: EventTransaction,
}

#[derive(serde::Deserialize, Debug)]
pub struct EventTransaction {
    pub message: EventMessage,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventMessage {
    pub account_keys: Vec<String>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventMeta {
    pub log_messages: Vec<String>,
    pub inner_instructions: Vec<EventInnerInstructions>,
}

#[derive(serde::Deserialize, Debug)]
pub struct EventInnerInstructions {
    pub instructions: Vec<EventAccountIndices>,
}

#[derive(serde::Deserialize, Debug)]
pub struct EventAccountIndices {
    pub accounts: Vec<u8>,
}

impl Handle {
    pub fn new() -> Result<Self, Error> {
        let helius_auth_token = helius_auth_token()?;
        let s = Self { helius_auth_token };
        Ok(s)
    }

    pub async fn rewards(
        handle: web::Data<Handle>,
        tx: web::Data<tokio::sync::mpsc::Sender<ore_api::event::MineEvent>>,
        req: HttpRequest,
        bytes: web::Bytes,
    ) -> impl Responder {
        let handle = handle.into_inner();
        match handle.handle_rewards_event(&req, &bytes, tx.as_ref()).await {
            Ok(_event) => HttpResponse::Ok().finish(),
            Err(err) => {
                log::error!("{:?}", err);
                let resp: HttpResponse = err.into();
                resp
            }
        }
    }

    async fn handle_rewards_event(
        &self,
        req: &HttpRequest,
        bytes: &web::Bytes,
        tx: &tokio::sync::mpsc::Sender<ore_api::event::MineEvent>,
    ) -> Result<(), Error> {
        let rewards = self.decode_rewards_event(req, bytes)?;
        tx.send(rewards).await?;
        Ok(())
    }

    fn decode_rewards_event(
        &self,
        req: &HttpRequest,
        bytes: &web::Bytes,
    ) -> Result<ore_api::event::MineEvent, Error> {
        self.auth(req)?;
        let bytes = bytes.to_vec();
        let json = serde_json::from_slice::<serde_json::Value>(bytes.as_slice())?;
        let event = serde_json::from_value::<Vec<Event>>(json)?;

        // parse the mine event
        let event = event
            .first()
            .ok_or(Error::Internal("empty webhook event".to_string()))?;
        let log_messages = event.meta.log_messages.as_slice();
        let index = log_messages.len().checked_sub(2).ok_or(Error::Internal(
            "invalid webhook event message index".to_string(),
        ))?;
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

    /// parse and validate the auth header
    fn auth(&self, req: &HttpRequest) -> Result<(), Error> {
        let header = req.headers().get("Authorization").ok_or(Error::Internal(
            "missing auth header in webhook event".to_string(),
        ))?;
        let header = header.to_str()?;
        if header.to_string().ne(&self.helius_auth_token) {
            return Err(Error::Internal(
                "invalid auth header in webhook event".to_string(),
            ));
        }
        Ok(())
    }
}

impl Client {
    /// create new client for listening to share account state changes
    pub fn new_stake() -> Result<Self, Error> {
        let helius_api_key = helius_api_key()?;
        let helius_webhook_id = helius_webhook_id()?;
        let helius_webhook_url = helius_webhook_url()?;
        let helius_auth_token = helius_auth_token()?;
        let s = Self {
            http_client: reqwest::Client::new(),
            helius_api_key,
            helius_webhook_id,
            helius_webhook_url,
            helius_auth_token,
        };
        Ok(s)
    }
}

/// this the /webhook path that your server exposes to helius.
fn helius_webhook_url() -> Result<String, Error> {
    std::env::var("HELIUS_WEBHOOK_URL").map_err(From::from)
}

fn helius_api_key() -> Result<String, Error> {
    std::env::var("HELIUS_API_KEY").map_err(From::from)
}

fn helius_auth_token() -> Result<String, Error> {
    std::env::var("HELIUS_AUTH_TOKEN").map_err(From::from)
}

fn helius_webhook_id() -> Result<String, Error> {
    std::env::var("HELIUS_WEBHOOK_ID").map_err(From::from)
}

#[cfg(test)]
mod tests {
    use ore_api::event::MineEvent;

    use super::*;

    #[test]
    fn test_mine_event() {
        let event = "Ex+oRr9TAAAlAAAAAAAAAB3PUGcAAAAA+P////////8A6HZIFwAAALC1wGMBAAAArkyHsAUAAABkC2efDAAAAA==";
        let event = BASE64_STANDARD.decode(event).unwrap();
        let event: &MineEvent = bytemuck::try_from_bytes(event.as_slice()).unwrap();
        println!("{:?}", event);
    }
}
