use actix_web::{web, HttpRequest, HttpResponse, Responder};
use base64::{prelude::BASE64_STANDARD, Engine};
use ore_pool_api::event::UnstakeEvent;
use solana_sdk::pubkey::Pubkey;

use crate::{aggregator::Aggregator, database, error::Error, operator::Operator};

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

pub struct ClientPutEntry {
    pub share: Pubkey,
    pub authority: Pubkey,
    pub mint: Pubkey,
}

/// the PUT edit payload, idempotent
#[derive(Debug, serde::Serialize)]
struct ClientEditPayload {
    #[serde(rename = "webhookURL")]
    webhook_url: String,
    #[serde(rename = "transactionTypes")]
    transaction_types: [String; 1],
    #[serde(rename = "accountAddresses")]
    pub account_addresses: Vec<String>,
    #[serde(rename = "webhookType")]
    webhook_type: String,
    #[serde(rename = "authHeader")]
    auth_header: String,
}

#[derive(serde::Deserialize, Debug)]
struct ClientEditSuccess {
    #[serde(rename = "webhookID")]
    webhook_id: String,
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

    pub async fn share_account(
        handle: web::Data<Handle>,
        aggregator: web::Data<tokio::sync::RwLock<Aggregator>>,
        req: HttpRequest,
        bytes: web::Bytes,
    ) -> impl Responder {
        let handle = handle.into_inner();
        match handle
            .handle_share_account_event(aggregator.as_ref(), &req, &bytes)
            .await
        {
            Ok(_event) => HttpResponse::Ok().finish(),
            Err(err) => {
                log::error!("{:?}", err);
                let resp: HttpResponse = err.into();
                resp
            }
        }
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

    async fn handle_share_account_event(
        &self,
        aggregator: &tokio::sync::RwLock<Aggregator>,
        req: &HttpRequest,
        bytes: &web::Bytes,
    ) -> Result<(), Error> {
        let mut event = self.decode_share_account_event(req, bytes).await?;
        self.process_share_account_event(aggregator, &mut event)
            .await?;
        Ok(())
    }

    /// decrement share account balance, only.
    /// increments are handled in the commit loop.
    /// this prevents an attack vector where stakers time increments,
    /// only to decrement again before the operator notices.
    async fn process_share_account_event(
        &self,
        aggregator: &tokio::sync::RwLock<Aggregator>,
        event: &mut UnstakeEvent,
    ) -> Result<(), Error> {
        let mut write = aggregator.write().await;
        let stake = &mut write.stake;
        let stakers = stake.get_mut(&event.mint).ok_or(Error::Internal(format!(
            "missing staker balances: {}",
            event.mint
        )))?;
        if let std::collections::hash_map::Entry::Occupied(ref mut occupied) =
            stakers.entry(event.authority)
        {
            let balance = occupied.get_mut();
            if balance > &mut event.balance {
                *balance = event.balance;
            }
        }
        Ok(())
    }

    /// decode the share account event.
    /// if cannot decode as unstake event respond with 200 ok
    /// so that helius server doesn't keep retrying.
    /// decoding here on our sever is internal to us,
    /// all helius needs to know is that we received the message.
    /// in fact, we should probably process on a new spawn
    /// and respond immediately to helius with an ok.
    async fn decode_share_account_event(
        &self,
        req: &HttpRequest,
        bytes: &web::Bytes,
    ) -> Result<UnstakeEvent, Error> {
        self.auth(req)?;
        let bytes = bytes.to_vec();
        let event = serde_json::from_slice::<Vec<Event>>(bytes.as_slice())?;
        // parse logs for updated balance
        // which sits in the 3rd to last line
        let event = event
            .first()
            .ok_or(Error::Internal("empty webhook event".to_string()))?;
        let log_messages = &event.meta.log_messages;
        let index = log_messages.len().checked_sub(3).ok_or(Error::Internal(
            "invalid webhook event message index".to_string(),
        ))?;
        let stake_event = log_messages
            .get(index)
            .ok_or(Error::Internal("missing webhook event message".to_string()))?;
        let stake_event = stake_event.trim_start_matches("Program data: ");
        let stake_event = BASE64_STANDARD
            .decode(stake_event)
            .map_err(|_| Error::ShareAccountReceived)?;
        let stake_event: &UnstakeEvent = bytemuck::try_from_bytes(stake_event.as_slice())
            .map_err(|_| Error::ShareAccountReceived)?;
        log::info!("share account webhook event: {:?}", stake_event);
        Ok(*stake_event)
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

    /// decodes event that is sent on mine instructions (by listening to the proof account).
    /// parses logs for base and boost rewards.
    fn decode_rewards_event(
        &self,
        req: &HttpRequest,
        bytes: &web::Bytes,
    ) -> Result<ore_api::event::MineEvent, Error> {
        self.auth(req)?;
        let bytes = bytes.to_vec();
        let json = serde_json::from_slice::<serde_json::Value>(bytes.as_slice())?;
        log::info!("{:?}", json);
        let event = serde_json::from_value::<Vec<Event>>(json)?;
        log::info!("proof account event: {:?}", event);
        let event = event
            .first()
            .ok_or(Error::Internal("empty webhook event".to_string()))?;
        let log_messages = event.meta.log_messages.as_slice();
        log::info!("logs: {:?}", log_messages);
        let index = log_messages.len().checked_sub(7).ok_or(Error::Internal(
            "invalid webhook event message index".to_string(),
        ))?;
        let mine_event = log_messages
            .get(index)
            .ok_or(Error::Internal("missing webhook base reward".to_string()))?;
        let mine_event =
            mine_event.trim_start_matches(format!("Program return: {} ", ore_api::ID).as_str());
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

    /// puts entry into webhook
    /// and marks in db
    pub async fn put(
        &self,
        operator: &Operator,
        aggregator: &tokio::sync::RwLock<Aggregator>,
        entry: &ClientPutEntry,
    ) -> Result<(), Error> {
        // lock
        let mut write = aggregator.write().await;
        // fetch db stakers
        let mut db_stakers: Vec<String> = vec![];
        for ba in operator.boost_accounts.iter() {
            let vec = operator.get_stakers_db_as_string(&ba.mint).await?;
            db_stakers.extend(vec);
        }
        // edit webhook
        let edit = self.edit(db_stakers).await?;
        log::info!("edit: {:?}", edit.webhook_id);
        // mark in db
        let db_client = &operator.db_client;
        let conn = db_client.get().await?;
        database::write_webhook_staker(&conn, &entry.share).await?;
        // insert into staker balancers
        let stake = &mut write.stake;
        let stakers = stake.get_mut(&entry.mint).ok_or(Error::Internal(format!(
            "missing staker balances: {}",
            entry.mint
        )))?;
        if let std::collections::hash_map::Entry::Vacant(vacant) = stakers.entry(entry.authority) {
            // insert as zero regardless of balance. increments are handled on submit loops.
            vacant.insert(0);
        }
        Ok(())
    }

    /// edit the listen-for accounts by passing the entire collection
    async fn edit(&self, account_addresses: Vec<String>) -> Result<ClientEditSuccess, Error> {
        let edit_url = format!(
            "{}/{}/{}?api-key={}",
            HELIUS_URL, HELIUS_WEBHOOK_API_PATH, self.helius_webhook_id, self.helius_api_key
        );
        let webhook_url = self.helius_webhook_url.clone();
        let auth_header = self.helius_auth_token.clone();
        let json = ClientEditPayload {
            account_addresses,
            transaction_types: [HELIUS_TRANSACTION_TYPE.to_string()],
            webhook_type: HELIUS_WEBHOOK_TYPE.to_string(),
            webhook_url,
            auth_header,
        };
        let resp = self
            .http_client
            .put(edit_url)
            .json(&json)
            .send()
            .await?
            .json::<ClientEditSuccess>()
            .await?;
        Ok(resp)
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
        let event = "JxveNQsJAAAIAAAAAAAAALbFEGcAAAAAaqcCAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==";
        let event = BASE64_STANDARD.decode(event).unwrap();
        let event: &MineEvent = bytemuck::try_from_bytes(event.as_slice()).unwrap();
        println!("{:?}", event);
    }
}
