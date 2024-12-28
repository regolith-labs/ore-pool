use actix_web::{web, HttpRequest, HttpResponse, Responder};
use base64::{prelude::BASE64_STANDARD, Engine};

use crate::error::Error;

/// handler for receiving helius webhook events
pub struct Handle {
    /// the auth token expected to be included in webhook events
    /// posted from helius to our server.
    helius_auth_token: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct Event {
    pub meta: EventMeta,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventMeta {
    pub log_messages: Vec<String>,
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
        // authorize
        self.auth(req)?;
        // decode payload
        let bytes = bytes.to_vec();
        let json = serde_json::from_slice::<serde_json::Value>(bytes.as_slice())?;
        let event = serde_json::from_value::<Vec<Event>>(json)?;
        // parse the mine event
        let event = event
            .first()
            .ok_or(Error::Internal("empty webhook event".to_string()))?;
        let log_messages = event.meta.log_messages.as_slice();
        let index = log_messages.len().checked_sub(5).ok_or(Error::Internal(
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

fn helius_auth_token() -> Result<String, Error> {
    std::env::var("HELIUS_AUTH_TOKEN").map_err(From::from)
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
