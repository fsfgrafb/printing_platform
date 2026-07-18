use chrono::Local;
use sqlx::SqlitePool;

use crate::{error::AppResult, services::settings};

pub fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

pub async fn daily_limit(pool: &SqlitePool) -> AppResult<i64> {
    let value = settings::get_or(pool, "daily_limit", "10").await?;
    Ok(value.parse::<i64>().unwrap_or(10).max(0))
}

pub async fn used_today(pool: &SqlitePool, user_id: i64) -> AppResult<i64> {
    let date = today();
    let used: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(page_count), 0)
        FROM print_tasks
        WHERE user_id = ?
          AND status = 'done'
          AND approved_over_quota = 0
          AND date(COALESCE(completed_at, submitted_at), 'localtime') = ?
        "#,
    )
    .bind(user_id)
    .bind(date)
    .fetch_one(pool)
    .await?;

    Ok(used)
}

pub async fn reserved(pool: &SqlitePool, user_id: i64) -> AppResult<i64> {
    Ok(sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(page_count), 0)
        FROM print_tasks
        WHERE user_id = ?
          AND status IN ('queued', 'spooling', 'printing')
          AND approved_over_quota = 0
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?)
}
