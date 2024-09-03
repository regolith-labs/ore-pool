use std::env;

use deadpool_postgres::{Object, Pool};
use ore_pool_api::state::member_pda;
use tokio_postgres::{Error, NoTls};

pub fn create_pool() -> Pool {
    let mut cfg = deadpool_postgres::Config::new();
    cfg.url = Some(env::var("DB_URL").expect("DB_URL must be set").to_string());
    cfg.create_pool(None, NoTls).unwrap()
}

pub async fn write_new_member(
    conn: &Object,
    member: &ore_pool_api::state::Member,
    approved: bool,
) -> Result<(), Error> {
    conn.execute(
        "INSERT INTO members
        (address, id, authority, pool_address, total_balance, is_approved, is_kyc)
        VALUES ($1, $2, $3, $4, $5, $6, $7)",
        &[
            &(member_pda(member.authority, member.pool).0.to_string()),
            &(member.id as i64),
            &member.authority.to_string(),
            &member.pool.to_string(),
            &0i64,
            &approved,
            &false,
        ],
    )
    .await?;
    Ok(())
}

pub async fn read_member(conn: &Object, address: &String) -> Result<types::Member, Error> {
    let row = conn
        .query_one(
            &format!(
                "SELECT address, id, authority, pool_address, total_balance, is_approved, is_kyc
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
    })
}
