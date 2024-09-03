use std::str::FromStr;

use actix_web::{web, HttpResponse, Responder};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use types::{ContributePayload, GetChallengePayload, GetMemberPayload, RegisterPayload};

use crate::{aggregator::Aggregator, database, error::Error, operator::Operator, tx, Contribution};

pub async fn register(
    operator: web::Data<Operator>,
    db_client: web::Data<deadpool_postgres::Pool>,
    payload: web::Json<RegisterPayload>,
) -> impl Responder {
    let operator = operator.as_ref();
    let res = register_new_member(operator, db_client.as_ref(), payload.into_inner()).await;
    match res {
        Ok(db_member) => HttpResponse::Ok().json(&db_member),
        Err(err) => {
            log::error!("{:?}", err);
            let http_response: HttpResponse = err.into();
            http_response
        }
    }
}

async fn register_new_member(
    operator: &Operator,
    db_client: &deadpool_postgres::Pool,
    payload: RegisterPayload,
) -> Result<types::Member, Error> {
    let payer = &operator.keypair;
    let member_authority = payload.authority;
    let (pool_pda, _) = ore_pool_api::state::pool_pda(payer.pubkey());
    // check if on-chain account already exists
    let member = operator.get_member(&member_authority).await;
    let rpc_client = &operator.rpc_client;
    let db_client = db_client.get().await?;
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
            // build ix
            let ix = ore_pool_api::instruction::open(member_authority, pool_pda, payer.pubkey());
            // submit and confirm
            let _ = tx::submit_and_confirm(payer, rpc_client, vec![ix], 1_000_000, 50_000).await?;
            // fetch member account for assigned id
            let member = operator.get_member(&member_authority).await?;
            // write member to db
            let db_member = database::write_new_member(&db_client, &member, false).await?;
            Ok(db_member)
        }
    }
}

pub async fn member(
    operator: web::Data<Operator>,
    db_client: web::Data<deadpool_postgres::Pool>,
    path: web::Path<GetMemberPayload>,
) -> impl Responder {
    match get_member(operator.as_ref(), db_client.as_ref(), path).await {
        Ok(member) => HttpResponse::Ok().json(&member),
        Err(err) => {
            log::error!("{:?}", err);
            HttpResponse::NotFound().finish()
        }
    }
}

async fn get_member(
    operator: &Operator,
    db_client: &deadpool_postgres::Pool,
    path: web::Path<GetMemberPayload>,
) -> Result<types::Member, Error> {
    let db_client = db_client.get().await?;
    let member_authority = path.into_inner();
    let member_authority = Pubkey::from_str(member_authority.authority.as_str())?;
    let pool_authority = operator.keypair.pubkey();
    let (pool_pda, _) = ore_pool_api::state::pool_pda(pool_authority);
    let (member_pda, _) = ore_pool_api::state::member_pda(member_authority, pool_pda);
    database::read_member(&db_client, &member_pda.to_string()).await
}

// type MemberId = u64;
// async fn register_return_data(rpc_client: &RpcClient, sig: &Signature) -> Result<MemberId, Error> {
//     let transaction = rpc_client
//         .get_transaction(
//             &sig,
//             solana_transaction_status::UiTransactionEncoding::JsonParsed,
//         )
//         .await?;
//     let return_data = transaction
//         .transaction
//         .meta
//         .ok_or(Error::Internal(
//             "missing return data (meta) from open instruction".to_string(),
//         ))?
//         .return_data;
//     let return_data: Option<UiTransactionReturnData> = From::from(return_data);
//     let (return_data, _) = return_data
//         .ok_or(Error::Internal(
//             "missing return data (meta) from open instruction".to_string(),
//         ))?
//         .data;
//     let return_data = BASE64_STANDARD.decode(return_data)?;
//     let return_data: [u8; 8] = return_data.as_slice().try_into()?;
//     let member_id = u64::from_le_bytes(return_data);
//     Ok(member_id)
// }

pub async fn challenge(
    payload: web::Path<GetChallengePayload>,
    aggregator: web::Data<tokio::sync::Mutex<Aggregator>>,
) -> impl Responder {
    let member_authority = payload.into_inner().authority;
    match Pubkey::from_str(member_authority.as_str()) {
        Ok(member_authority) => {
            let aggregator = aggregator.as_ref();
            let mut aggregator = aggregator.lock().await;
            let challenge = aggregator.nonce_index(&member_authority).await;
            match challenge {
                Ok(challenge) => HttpResponse::Ok().json(challenge),
                Err(err) => {
                    log::error!("{:?}", err);
                    HttpResponse::InternalServerError().finish()
                }
            }
        }
        Err(err) => {
            log::error!("{:?}", err);
            HttpResponse::BadRequest().body(err.to_string())
        }
    }
}

/// Accepts solutions from pool members. If their solutions are valid, it
/// aggregates the contributions into a list for publishing and submission.
pub async fn contribute(
    payload: web::Json<ContributePayload>,
    tx: web::Data<tokio::sync::mpsc::UnboundedSender<Contribution>>,
    aggregator: web::Data<tokio::sync::Mutex<Aggregator>>,
) -> impl Responder {
    log::info!("received payload");
    log::info!("decoded: {:?}", payload);
    // lock aggregrator to ensure we're contributing to the current challenge
    let aggregator = aggregator.as_ref();
    let aggregator = aggregator.lock().await;
    // decode solution difficulty
    let solution = &payload.solution;
    log::info!("solution: {:?}", solution);
    let difficulty = solution.to_hash().difficulty();
    log::info!("difficulty: {:?}", difficulty);
    // authenticate the sender signature
    if !payload
        .signature
        .verify(&payload.authority.to_bytes(), &solution.to_bytes())
    {
        return HttpResponse::Unauthorized().finish();
    }
    // error if solution below min difficulty
    if difficulty < (aggregator.challenge.min_difficulty as u32) {
        log::error!("solution below min difficulity: {:?}", payload.authority);
        return HttpResponse::BadRequest().finish();
    }
    // error if digest is invalid
    if !drillx::is_valid_digest(&aggregator.challenge.challenge, &solution.n, &solution.d) {
        log::error!("invalid solution");
        return HttpResponse::BadRequest().finish();
    }
    // calculate score
    let score = 2u64.pow(difficulty);
    log::info!("score: {}", score);
    // TODO: Reject if score is below min difficulty (as defined by the pool operator)

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
