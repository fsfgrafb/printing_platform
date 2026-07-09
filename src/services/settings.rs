use sqlx::SqlitePool;

use crate::error::AppResult;

pub async fn get(pool: &SqlitePool, key: &str) -> AppResult<Option<String>> {
    Ok(
        sqlx::query_scalar("SELECT value FROM global_config WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn get_or(pool: &SqlitePool, key: &str, default: &str) -> AppResult<String> {
    Ok(get(pool, key).await?.unwrap_or_else(|| default.to_string()))
}

pub async fn set(pool: &SqlitePool, key: &str, value: &str) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO global_config (key, value)
        VALUES (?, ?)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn queue_paused(pool: &SqlitePool) -> AppResult<bool> {
    Ok(get_or(pool, "queue_paused", "false").await? == "true")
}

pub async fn set_queue_paused(pool: &SqlitePool, paused: bool) -> AppResult<()> {
    set(pool, "queue_paused", if paused { "true" } else { "false" }).await
}
