use std::env;

use crate::error::Error;
use deadpool_postgres::{Object, Pool};
use ore_pool_api::state::member_pda;
use tokio_postgres::NoTls;

pub fn create_pool() -> Pool {
    let mut cfg = deadpool_postgres::Config::new();
    cfg.url = Some(env::var("DB_URL").expect("DB_URL must be set").to_string());
    cfg.create_pool(None, NoTls).unwrap()
}

// TODO: update balance
pub async fn write_new_member(
    conn: &Object,
    member: &ore_pool_api::state::Member,
    approved: bool,
) -> Result<types::Member, Error> {
    let member = types::Member {
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

pub async fn read_member(conn: &Object, address: &String) -> Result<types::Member, Error> {
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
    Ok(types::Member {
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
