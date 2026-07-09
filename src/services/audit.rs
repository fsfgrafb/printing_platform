use serde::Serialize;
use sqlx::SqlitePool;

use crate::error::AppResult;

pub async fn log<T: Serialize>(
    pool: &SqlitePool,
    user_id: Option<i64>,
    action: &str,
    details: &T,
) -> AppResult<()> {
    let details = serde_json::to_string(details).unwrap_or_else(|_| "{}".to_string());
    sqlx::query("INSERT INTO audit_log (user_id, action, details) VALUES (?, ?, ?)")
        .bind(user_id)
        .bind(action)
        .bind(details)
        .execute(pool)
        .await?;
    Ok(())
}
