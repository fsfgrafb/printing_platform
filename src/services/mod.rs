pub mod audit;
pub mod cleanup;
pub mod converter;
pub mod import;
pub mod print_service;
pub mod printer;
pub mod queue_manager;
pub mod quota;

pub mod settings {
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

    pub async fn increment_counter(pool: &SqlitePool, key: &str) -> AppResult<i64> {
        Ok(sqlx::query_scalar(
            r#"
            INSERT INTO global_config (key, value)
            VALUES (?, '1')
            ON CONFLICT(key) DO UPDATE
            SET value = CAST(global_config.value AS INTEGER) + 1
            RETURNING CAST(value AS INTEGER)
            "#,
        )
        .bind(key)
        .fetch_one(pool)
        .await?)
    }

    pub async fn initialize_daily_limit(pool: &SqlitePool, default_limit: i64) -> AppResult<()> {
        const MARKER: &str = "default_daily_limit_config_v1";
        if get(pool, MARKER).await?.is_some() {
            return Ok(());
        }

        let current = get(pool, "daily_limit").await?;
        if current.is_none() || current.as_deref() == Some("50") {
            set(pool, "daily_limit", &default_limit.max(0).to_string()).await?;
        }
        set(pool, MARKER, "true").await
    }

    pub async fn queue_paused(pool: &SqlitePool) -> AppResult<bool> {
        Ok(get_or(pool, "queue_paused", "false").await? == "true")
    }

    pub async fn set_queue_paused(pool: &SqlitePool, paused: bool) -> AppResult<()> {
        set(pool, "queue_paused", if paused { "true" } else { "false" }).await
    }
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::settings;

    #[tokio::test]
    async fn configured_daily_limit_initializes_new_and_legacy_databases_once() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE global_config (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();

        settings::initialize_daily_limit(&pool, 10).await.unwrap();
        assert_eq!(
            settings::get(&pool, "daily_limit")
                .await
                .unwrap()
                .as_deref(),
            Some("10")
        );
        settings::initialize_daily_limit(&pool, 20).await.unwrap();
        assert_eq!(
            settings::get(&pool, "daily_limit")
                .await
                .unwrap()
                .as_deref(),
            Some("10")
        );

        sqlx::query("DELETE FROM global_config")
            .execute(&pool)
            .await
            .unwrap();
        settings::set(&pool, "daily_limit", "50").await.unwrap();
        settings::initialize_daily_limit(&pool, 10).await.unwrap();
        assert_eq!(
            settings::get(&pool, "daily_limit")
                .await
                .unwrap()
                .as_deref(),
            Some("10")
        );

        sqlx::query("DELETE FROM global_config")
            .execute(&pool)
            .await
            .unwrap();
        settings::set(&pool, "daily_limit", "25").await.unwrap();
        settings::initialize_daily_limit(&pool, 10).await.unwrap();
        assert_eq!(
            settings::get(&pool, "daily_limit")
                .await
                .unwrap()
                .as_deref(),
            Some("25")
        );
    }

    #[tokio::test]
    async fn counters_are_created_and_incremented_atomically() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE global_config (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(
            settings::increment_counter(&pool, "submit_page_visits")
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            settings::increment_counter(&pool, "submit_page_visits")
                .await
                .unwrap(),
            2
        );
    }
}
