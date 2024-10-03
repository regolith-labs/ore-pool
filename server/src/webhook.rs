use std::collections::HashMap;

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use solana_sdk::pubkey::Pubkey;

use crate::{aggregator::Aggregator, database, error::Error, operator::Operator};

const HELIUS_URL: &str = "https://api.helius.xyz";
const HELIUS_WEBHOOK_API_PATH: &str = "v0/webhooks";

/// client for managing helius webhooks
pub struct Client {
    http_client: reqwest::Client,
    /// query paramter added to the url for making http requets to helius.
    helius_api_key: String,
    /// the helius webhook id created in the console
    /// for tracking share accounts
    helius_webhook_id_stake: String,
}

/// handler for receiving helius webhook events
pub struct Handle {
    /// the auth token expected to be included in webhook events
    /// posted from helius to our server.
    helius_auth_token: String,
}

#[derive(serde::Deserialize, Debug)]
struct EditSuccess {
    #[serde(rename = "webhookID")]
    webhook_id: String,
}

#[derive(serde::Deserialize, Debug)]
struct ShareAccountEvent {}

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
        let event = serde_json::from_slice::<ShareAccountEvent>(bytes.as_slice())?;
        log::info!("share account event: {:?}", event);
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

pub struct ClientPutEntry {
    pub share: Pubkey,
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub share_account: ore_pool_api::state::Share,
}

impl Client {
    /// create new client for listening to share account state changes
    pub fn new_stake() -> Result<Self, Error> {
        let helius_api_key = helius_api_key()?;
        let helius_webhook_id_stake = helius_webhook_id_stake()?;
        let s = Self {
            http_client: reqwest::Client::new(),
            helius_api_key,
            helius_webhook_id_stake,
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
        let db_stakers = operator.get_stakers_db(&entry.mint).await?;
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
    async fn edit(&self, account_addresses: Vec<Pubkey>) -> Result<EditSuccess, Error> {
        // const response = await fetch('https://api.helius.xyz/v0/webhooks/{webhookID}?api-key=text', {
        let edit_url = format!(
            "{}/{}/{}?{}",
            HELIUS_URL, HELIUS_WEBHOOK_API_PATH, self.helius_webhook_id_stake, self.helius_api_key
        );
        let mut json = HashMap::<String, Vec<Pubkey>>::new();
        json.insert("accountAddresses".to_string(), account_addresses);
        let resp = self
            .http_client
            .put(edit_url)
            .json(&json)
            .send()
            .await?
            .json::<EditSuccess>()
            .await?;
        Ok(resp)
    }
}

fn helius_api_key() -> Result<String, Error> {
    std::env::var("HELIUS_API_KEY").map_err(From::from)
}

fn helius_auth_token() -> Result<String, Error> {
    std::env::var("HELIUS_AUTH_TOAUTH_TOKEN").map_err(From::from)
}

fn helius_webhook_id_stake() -> Result<String, Error> {
    std::env::var("HELIUS_WEBHOOK_ID_STAKE").map_err(From::from)
}
