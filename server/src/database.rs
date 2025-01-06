use std::{env, str::FromStr, sync::Arc};

use crate::{error::Error, operator::Operator, tx};
use deadpool_postgres::{GenericClient, Object, Pool};
use futures::TryStreamExt;
use futures_util::pin_mut;
use ore_pool_api::state::member_pda;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signer::Signer};
use tokio_postgres::NoTls;

pub fn create_pool() -> Pool {
    let mut cfg = deadpool_postgres::Config::new();
    cfg.url = Some(env::var("DB_URL").expect("DB_URL must be set").to_string());
    cfg.create_pool(None, NoTls).unwrap()
}

// when writing new balances
// also sets the is-synced field to false
// so that in the attribution loop we know which accounts
// have been incremented in the db but not yet on-chain
pub async fn update_member_balances(
    conn: &mut Object,
    increments: Vec<(Pubkey, u64)>,
) -> Result<(), Error> {
    let transaction = conn.transaction().await?;
    for (address, increment) in increments.iter() {
        transaction
                .execute(
                    "UPDATE members SET total_balance = total_balance + $1, is_synced = false WHERE authority = $2",
                    &[&(*increment as i64), &address.to_string()],
                )
                .await?;
    }
    transaction.commit().await?;
    Ok(())
}

// streams all records from db where is-synced is false
// updates on-chain balances in batches and marks records in db as synced,
// the on-chain attribution instruction is idempotent
// so any failures here are recoverable
const NUM_ATTRIBUTIONS_PER_TX: usize = 20;
pub async fn stream_members_attribution(
    conn: Arc<Object>,
    operator: Arc<Operator>,
) -> Result<(), Error> {
    // fetch count(*) to determine min buffer size
    let count_query = "SELECT COUNT(*) FROM members WHERE is_synced = false";
    let row = conn.query_one(count_query, &[]).await?;
    let record_count: i64 = row.try_get(0)?;

    // build stream of memebrs to be attributed
    let stmt = "SELECT address, authority, total_balance FROM members WHERE is_synced = false";
    let params: Vec<String> = vec![];
    let stream = conn.query_raw(stmt, params).await?;
    pin_mut!(stream);

    // buffer stream for packing attributions transaction
    let signer = operator.keypair.pubkey();
    let buffer_size = NUM_ATTRIBUTIONS_PER_TX.min(record_count as usize);
    let mut ix_buffer: Vec<Instruction> = Vec::with_capacity(buffer_size);
    let mut address_buffer: Vec<String> = Vec::with_capacity(buffer_size);
    let mut handles: Vec<tokio::task::JoinHandle<()>> = vec![];
    while let Some(row) = stream.try_next().await? {
        // parse row
        let address: String = row.try_get(0)?;
        let member_authority: String = row.try_get(1)?;
        let member_authority = Pubkey::from_str(member_authority.as_str())?;
        let total_balance: i64 = row.try_get(2)?;

        // build instruction
        let ix = ore_pool_api::sdk::attribute(signer, member_authority, total_balance as u64);
        ix_buffer.push(ix);
        address_buffer.push(address);

        // if buffer is full
        if ix_buffer.len().eq(&buffer_size) {
            let conn = conn.clone();
            let operator = operator.clone();
            let handle = tokio::spawn({
                let ix_buffer = ix_buffer.clone();
                let address_buffer = address_buffer.clone();
                async move {
                    match tx::submit::submit_and_confirm_instructions(
                        &operator.keypair,
                        &operator.rpc_client,
                        ix_buffer.as_slice(),
                        30_000,
                        20_000,
                    )
                    .await
                    {
                        Ok(sig) => {
                            log::info!("attribution sig: {:?}", sig);
                            // mark as synced
                            if let Err(err) =
                                write_synced_members(conn.as_ref(), address_buffer.as_slice()).await
                            {
                                log::error!("{:?}", err);
                            }
                        }
                        Err(err) => {
                            log::error!("{:?}", err);
                        }
                    }
                }
            });
            handles.push(handle);
            
            // clear buffers
            address_buffer.clear();
            ix_buffer.clear();
        }
    }

    // join handles
    let _ = futures::future::join_all(handles).await;
    Ok(())
}

pub async fn write_synced_members(conn: &Object, address_buffer: &[String]) -> Result<(), Error> {
    let query = "UPDATE members SET is_synced = true WHERE address = ANY($1)";
    conn.execute(query, &[&address_buffer]).await?;
    Ok(())
}

pub async fn write_new_member(
    conn: &Object,
    member: &ore_pool_api::state::Member,
    approved: bool,
) -> Result<ore_pool_types::Member, Error> {
    let member = ore_pool_types::Member {
        address: member_pda(member.authority, member.pool).0.to_string(),
        id: (member.id as i64),
        authority: member.authority.to_string(),
        pool_address: member.pool.to_string(),
        total_balance: 0,
        is_approved: approved,
        is_kyc: false,
        is_synced: true,
    };
    conn.execute(
        "INSERT INTO members
        (address, id, authority, pool_address, total_balance, is_approved, is_kyc, is_synced)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        &[
            &member.address,
            &member.id,
            &member.authority,
            &member.pool_address,
            &member.total_balance,
            &member.is_approved,
            &member.is_kyc,
            &member.is_synced,
        ],
    )
    .await?;
    Ok(member)
}

pub async fn read_member(conn: &Object, address: &String) -> Result<ore_pool_types::Member, Error> {
    let row = conn
        .query_one(
            &format!(
                "SELECT address, id, authority, pool_address, total_balance, is_approved, is_kyc, is_synced
                FROM members
                WHERE address = '{}'",
                address
            ),
            &[],
        )
        .await?;
    Ok(ore_pool_types::Member {
        address: row.try_get(0)?,
        id: row.try_get(1)?,
        authority: row.try_get(2)?,
        pool_address: row.try_get(3)?,
        total_balance: row.try_get(4)?,
        is_approved: row.try_get(5)?,
        is_kyc: row.try_get(6)?,
        is_synced: row.try_get(7)?,
    })
}
