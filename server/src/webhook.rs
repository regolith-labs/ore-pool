use actix_web::{web, HttpRequest, HttpResponse, Responder};
use base64::{prelude::BASE64_STANDARD, Engine};
use ore_pool_api::event::StakeEvent;
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
    helius_webhook_id_stake: String,
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
    pub share_account: ore_pool_api::state::Share,
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
struct ShareAccountEvent {
    pub meta: ShareAccountEventMeta,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ShareAccountEventMeta {
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
        req: HttpRequest,
        bytes: web::Bytes,
    ) -> impl Responder {
        let handle = handle.into_inner();
        match handle.handle_share_account_event(&req, &bytes).await {
            Ok(()) => HttpResponse::Ok().finish(),
            Err(err) => {
                log::error!("{:?}", err);
                HttpResponse::InternalServerError().body(err.to_string())
            }
        }
    }

    /// process the share account event
    async fn handle_share_account_event(
        &self,
        req: &HttpRequest,
        bytes: &web::Bytes,
    ) -> Result<(), Error> {
        self.auth(req)?;
        let bytes = bytes.to_vec();
        let event = serde_json::from_slice::<Vec<ShareAccountEvent>>(bytes.as_slice())?;
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
        let stake_event = stake_event.trim_start_matches("Program log: ");
        let stake_event = BASE64_STANDARD.decode(stake_event)?;
        let stake_event: &StakeEvent = bytemuck::try_from_bytes(stake_event.as_slice())
            .map_err(|err| Error::Internal(err.to_string()))?;
        log::info!("share account webhook event: {:?}", stake_event);
        Ok(())
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
        let helius_webhook_id_stake = helius_webhook_id_stake()?;
        let helius_webhook_url = helius_webhook_url()?;
        let helius_auth_token = helius_auth_token()?;
        let s = Self {
            http_client: reqwest::Client::new(),
            helius_api_key,
            helius_webhook_id_stake,
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
        let db_stakers = operator.get_stakers_db_as_string(&entry.mint).await?;
        // edit webhook
        let edit = self.edit(db_stakers).await?;
        log::info!("edit: {:?}", edit.webhook_id);
        // mark in db
        let db_client = &operator.db_client;
        let conn = db_client.get().await?;
        database::write_webhook_staker(&conn, &entry.share).await?;
        // insert into staker balancers
        let stakers = &mut write.stake;
        if let std::collections::hash_map::Entry::Vacant(vacant) = stakers.entry(entry.authority) {
            vacant.insert(entry.share_account.balance);
        }
        Ok(())
    }

    /// edit the listen-for accounts by passing the entire collection
    async fn edit(&self, account_addresses: Vec<String>) -> Result<ClientEditSuccess, Error> {
        let edit_url = format!(
            "{}/{}/{}?api-key={}",
            HELIUS_URL, HELIUS_WEBHOOK_API_PATH, self.helius_webhook_id_stake, self.helius_api_key
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

fn helius_webhook_id_stake() -> Result<String, Error> {
    std::env::var("HELIUS_WEBHOOK_ID_STAKE").map_err(From::from)
}
