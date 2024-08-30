use std::env;

use deadpool_postgres::{Object, Pool};
use ore_pool_api::state::{member_pda, Member};
use tokio_postgres::{Error, NoTls};

pub fn create_pool() -> Pool {
    let mut cfg = deadpool_postgres::Config::new();
    cfg.url = Some(env::var("DB_URL").expect("DB_URL must be set").to_string());
    cfg.create_pool(None, NoTls).unwrap()
}

pub async fn write_new_member(conn: &Object, member: &Member, approved: bool) -> Result<(), Error> {
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
