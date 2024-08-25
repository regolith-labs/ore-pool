use std::env;

use deadpool_postgres::{Object, Pool};
use ore_pool_api::state::{member_pda, Member};
use tokio_postgres::{Error, NoTls};

pub fn create_pool() -> Pool {
    let mut cfg = deadpool_postgres::Config::new();
    cfg.url = Some(env::var("DBURL").expect("DBURL must be set").to_string());
    cfg.create_pool(None, NoTls).unwrap()
}

pub async fn write_member(conn: &Object, member: Member) -> Result<(), Error> {
    conn.execute(
        "INSERT INTO members
        (address, authority, balance, id, is_approved, is_kyc)
        VALUES ($1, $2, $3, $4, $5, $6)",
        &[
            &member_pda(member.authority, member.pool).0.to_string(),
            &member.authority.to_string(),
            &(member.balance as i64),
            &(member.id as i64),
            &false,
            &false,
        ],
    )
    .await?;
    Ok(())
}
