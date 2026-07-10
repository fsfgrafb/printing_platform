use serde::Serialize;
use sqlx::SqlitePool;

use crate::error::AppResult;

pub async fn log<T: Serialize>(
    pool: &SqlitePool,
    user_id: Option<i64>,
    action: &str,
    details: &T,
) -> AppResult<()> {
    log_with_ip(pool, user_id, action, details, None).await
}

pub async fn log_with_ip<T: Serialize>(
    pool: &SqlitePool,
    user_id: Option<i64>,
    action: &str,
    details: &T,
    ip: Option<&str>,
) -> AppResult<()> {
    let details = serde_json::to_string(details).unwrap_or_else(|_| "{}".to_string());
    sqlx::query("INSERT INTO audit_log (user_id, action, details, ip) VALUES (?, ?, ?, ?)")
        .bind(user_id)
        .bind(action)
        .bind(details)
        .bind(ip)
        .execute(pool)
        .await?;
    Ok(())
}
