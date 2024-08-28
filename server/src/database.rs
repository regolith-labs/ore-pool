use std::env;

use deadpool_postgres::{Object, Pool};
use ore_pool_api::state::{member_pda, Member};
use tokio_postgres::{Error, NoTls};

pub fn create_pool() -> Pool {
    let mut cfg = deadpool_postgres::Config::new();
    cfg.url = Some(env::var("DB_URL").expect("DB_URL must be set").to_string());
    cfg.create_pool(None, NoTls).unwrap()
}

async fn _write_member(conn: &Object, member: Member) -> Result<(), Error> {
    conn.execute(
        "INSERT INTO members
        (address, authority, balance, is_approved, is_kyc)
        VALUES ($1, $2, $3, $4, $5)",
        &[
            &member_pda(member.authority, member.pool).0.to_string(),
            &member.authority.to_string(),
            &(member.balance as i64),
            &false,
            &false,
        ],
    )
    .await?;
    Ok(())
}
