use actix_web::{web, HttpResponse, Responder};
use ore_pool_types::{
    BalanceUpdate, ContributePayload, GetMemberPayload, MemberChallenge, PoolAddress,
    RegisterPayload, RegisterStakerPayload, Staker, UpdateBalancePayload,
};
use solana_sdk::{pubkey::Pubkey, signer::Signer};

use crate::{
    aggregator::{Aggregator, BUFFER_CLIENT},
    database,
    error::Error,
    operator::Operator,
    tx, webhook, Contribution,
};

////////////////////////////////////////////////////////////////////////////////////
/// HTTP HANDLERS //////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////////
pub async fn register(
    operator: web::Data<Operator>,
    payload: web::Json<RegisterPayload>,
) -> impl Responder {
    let operator = operator.as_ref();
    let res = register_new_member(operator, payload.into_inner()).await;
    match res {
        Ok(db_member) => HttpResponse::Ok().json(&db_member),
        Err(err) => {
            log::error!("{:?}", err);
            let http_response: HttpResponse = err.into();
            http_response
        }
    }
}

pub async fn register_staker(
    operator: web::Data<Operator>,
    aggregator: web::Data<tokio::sync::RwLock<Aggregator>>,
    webhook_client: web::Data<webhook::Client>,
    payload: web::Json<RegisterStakerPayload>,
) -> impl Responder {
    let res = register_new_staker(
        operator.as_ref(),
        aggregator.as_ref(),
        webhook_client.as_ref(),
        payload.into_inner(),
    )
    .await;
    match res {
        Ok(staker) => HttpResponse::Ok().json(staker),
        Err(err) => {
            log::error!("{:?}", err);
            let http_response: HttpResponse = err.into();
            http_response
        }
    }
}

pub async fn pool_address(operator: web::Data<Operator>) -> impl Responder {
    let operator = operator.as_ref();
    let (pool_pda, bump) = ore_pool_api::state::pool_pda(operator.keypair.pubkey());
    HttpResponse::Ok().json(&PoolAddress {
        address: pool_pda,
        bump,
    })
}

pub async fn update_balance(
    operator: web::Data<Operator>,
    payload: web::Json<UpdateBalancePayload>,
) -> impl Responder {
    match update_balance_onchain(operator.as_ref(), payload.into_inner()).await {
        Ok(balance_update) => HttpResponse::Ok().json(balance_update),
        Err(err) => {
            log::error!("{:?}", err);
            HttpResponse::InternalServerError().body(err.to_string())
        }
    }
}

pub async fn member(
    operator: web::Data<Operator>,
    path: web::Path<GetMemberPayload>,
) -> impl Responder {
    match operator
        .get_member_db(path.into_inner().authority.as_str())
        .await
    {
        Ok(member) => HttpResponse::Ok().json(&member),
        Err(err) => {
            log::error!("{:?}", err);
            HttpResponse::NotFound().finish()
        }
    }
}

// TODO: consider the need for auth on this get/read?
pub async fn challenge(aggregator: web::Data<tokio::sync::RwLock<Aggregator>>) -> impl Responder {
    // acquire read on aggregator for challenge
    let (challenge, last_num_members) = {
        let aggregator = aggregator.read().await;
        (aggregator.challenge, aggregator.num_members)
    };
    // build member challenge
    let member_challenge = MemberChallenge {
        challenge,
        buffer: BUFFER_CLIENT,
        num_total_members: last_num_members,
    };
    HttpResponse::Ok().json(&member_challenge)
}

/// Accepts solutions from pool members. If their solutions are valid, it
/// aggregates the contributions into a list for publishing and submission.
pub async fn contribute(
    operator: web::Data<Operator>,
    aggregator: web::Data<tokio::sync::RwLock<Aggregator>>,
    tx: web::Data<tokio::sync::mpsc::UnboundedSender<Contribution>>,
    payload: web::Json<ContributePayload>,
) -> impl Responder {
    // acquire read on aggregator for challenge
    let aggregator = aggregator.read().await;
    let challenge = aggregator.challenge;
    let num_members = aggregator.num_members;
    drop(aggregator);
    // decode solution difficulty
    let solution = &payload.solution;
    let difficulty = solution.to_hash().difficulty();
    // authenticate the sender signature
    if !payload
        .signature
        .verify(&payload.authority.to_bytes(), &solution.to_bytes())
    {
        return HttpResponse::Unauthorized().finish();
    }
    // error if solution below min difficulty
    if difficulty < (challenge.min_difficulty as u32) {
        log::error!("solution below min difficulity: {:?}", payload.authority);
        return HttpResponse::BadRequest().finish();
    }
    // validate nonce
    let member_authority = &payload.authority;
    let nonce = solution.n;
    let nonce = u64::from_le_bytes(nonce);
    if let Err(err) = validate_nonce(operator.as_ref(), member_authority, nonce, num_members).await
    {
        log::error!("{:?}", err);
        return HttpResponse::Unauthorized().finish();
    }
    // calculate score
    let score = 2u64.pow(difficulty);
    // update the aggegator
    if let Err(err) = tx.send(Contribution {
        member: payload.authority,
        score,
        solution: payload.solution,
    }) {
        log::error!("{:?}", err);
    }
    HttpResponse::Ok().finish()
}
////////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////////

async fn update_balance_onchain(
    operator: &Operator,
    payload: UpdateBalancePayload,
) -> Result<BalanceUpdate, Error> {
    let keypair = &operator.keypair;
    let member_authority = payload.authority;
    let hash = payload.hash;
    // fetch member balance
    let member = operator
        .get_member_db(member_authority.to_string().as_str())
        .await?;
    // assert that the fee payer is someone else
    let tx = payload.transaction;
    let fee_payer = tx.message.account_keys.first().ok_or(Error::Internal(
        "missing fee payer in update balance payload".to_string(),
    ))?;
    if fee_payer.eq(&keypair.pubkey()) {
        return Err(Error::Internal(
            "fee payer must be client for update balance".to_string(),
        ));
    }
    // validate transaction
    tx::validate::validate_attribution(&tx, member.total_balance)?;
    // sign transaction and submit
    let mut tx = tx;
    let rpc_client = &operator.rpc_client;
    tx.partial_sign(&[keypair], hash);
    let sig = tx::submit::submit_and_confirm_transaction(rpc_client, &tx).await?;
    log::info!("on demand attribution sig: {:?}", sig);
    // set member as synced in db
    let db_client = &operator.db_client;
    let db_client = db_client.get().await?;
    let (pool_address, _) = ore_pool_api::state::pool_pda(keypair.pubkey());
    let (member_address, _) = ore_pool_api::state::member_pda(member_authority, pool_address);
    database::write_synced_members(&db_client, &[member_address.to_string()]).await?;
    Ok(BalanceUpdate {
        balance: member.total_balance as u64,
        signature: sig,
    })
}

async fn register_new_staker(
    operator: &Operator,
    aggregator: &tokio::sync::RwLock<Aggregator>,
    webhook_client: &webhook::Client,
    payload: RegisterStakerPayload,
) -> Result<Staker, Error> {
    let keypair = &operator.keypair;
    let member_authority = payload.authority;
    let mint = payload.mint;
    // check if on-chain account already exists
    let staker = operator.get_staker_onchain(&member_authority, &mint).await;
    match staker {
        Ok(staker) => {
            // staker already exists on-chain
            // check if record is in db already
            let db_staker = operator.get_staker_db(&member_authority, &mint).await;
            match db_staker {
                Ok(db_staker) => {
                    // check if marked as added to webhook in db
                    if !db_staker.webhook {
                        // add to webhook
                        let entry = webhook::ClientPutEntry {
                            share: staker.1,
                            authority: member_authority,
                            mint,
                        };
                        webhook_client.put(operator, aggregator, &entry).await?;
                    }
                    Ok(db_staker)
                }
                Err(_err) => {
                    // write staker to db
                    let conn = operator.db_client.get().await?;
                    let (pool_pda, _) = ore_pool_api::state::pool_pda(keypair.pubkey());
                    let db_staker =
                        database::write_new_staker(&conn, &member_authority, &pool_pda, &mint)
                            .await?;
                    // add to webhook
                    let entry = webhook::ClientPutEntry {
                        share: staker.1,
                        authority: member_authority,
                        mint,
                    };
                    webhook_client.put(operator, aggregator, &entry).await?;
                    Ok(db_staker)
                }
            }
        }
        Err(err) => {
            // staker doesn't exist yet on-chain
            log::error!("{:?}", err);
            // return error to http client
            // bc they should create the staker (share) account before hitting this path
            Err(Error::StakerDoesNotExist)
        }
    }
}

async fn register_new_member(
    operator: &Operator,
    payload: RegisterPayload,
) -> Result<ore_pool_types::Member, Error> {
    let keypair = &operator.keypair;
    let member_authority = payload.authority;
    let (pool_pda, _) = ore_pool_api::state::pool_pda(keypair.pubkey());
    // check if on-chain account already exists
    let member = operator.get_member_onchain(&member_authority).await;
    let db_client = operator.db_client.get().await?;
    match member {
        Ok(member) => {
            // member already exists on-chain
            // check if record is in db already
            let (member_pda, _) = ore_pool_api::state::member_pda(member_authority, pool_pda);
            let db_member = database::read_member(&db_client, &member_pda.to_string()).await;
            match db_member {
                Ok(db_member) => {
                    // member already exists in db
                    Ok(db_member)
                }
                Err(_) => {
                    // write member to db
                    let db_member = database::write_new_member(&db_client, &member, false).await?;
                    Ok(db_member)
                }
            }
        }
        Err(err) => {
            // member doesn't exist yet on-chain
            // land tx to create new member account
            log::error!("{:?}", err);
            // return error to http client
            // bc they should create the member account before hitting this path
            Err(Error::MemberDoesNotExist)
        }
    }
}

// TODO: consider fitting lookup table from member authority to id, in memory
async fn validate_nonce(
    operator: &Operator,
    member_authority: &Pubkey,
    nonce: u64,
    num_members: u64,
) -> Result<(), Error> {
    if num_members.eq(&0) {
        return Ok(());
    }
    let member = operator
        .get_member_db(member_authority.to_string().as_str())
        .await?;
    let nonce_index = member.id as u64;
    let u64_unit = u64::MAX.saturating_div(num_members);
    let left_bound = u64_unit.saturating_mul(nonce_index);
    let right_bound = u64_unit.saturating_mul(nonce_index + 1);
    let ge_left = nonce >= left_bound;
    let le_right = nonce <= right_bound;
    if ge_left && le_right {
        Ok(())
    } else {
        Err(Error::Internal("invalid nonce from client".to_string()))
    }
}
