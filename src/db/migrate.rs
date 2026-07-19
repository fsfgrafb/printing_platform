use sqlx::{Executor, SqlitePool};

use crate::error::AppResult;

pub async fn run(pool: &SqlitePool) -> AppResult<()> {
    let statements = [
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            student_id TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            qq TEXT,
            phone TEXT,
            status TEXT NOT NULL DEFAULT 'unused',
            must_change_password INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            last_login_at TEXT
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            token TEXT NOT NULL UNIQUE,
            user_id INTEGER NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            expires_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS temp_uploads (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            temp_id TEXT NOT NULL UNIQUE,
            user_id INTEGER NOT NULL,
            original_name TEXT NOT NULL,
            stored_path TEXT NOT NULL,
            preview_path TEXT NOT NULL,
            page_count INTEGER NOT NULL DEFAULT 1,
            byte_size INTEGER NOT NULL DEFAULT 0,
            content_hash TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS print_tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            file_name TEXT NOT NULL,
            stored_path TEXT NOT NULL,
            preview_path TEXT,
            page_count INTEGER NOT NULL DEFAULT 0,
            source_page_count INTEGER,
            content_hash TEXT,
            odd_even TEXT NOT NULL DEFAULT 'all',
            status TEXT NOT NULL DEFAULT 'queued',
            submitted_at TEXT NOT NULL DEFAULT (datetime('now')),
            completed_at TEXT,
            cancelled_by TEXT,
            review_reason TEXT,
            approved_over_quota INTEGER NOT NULL DEFAULT 0,
            windows_job_id INTEGER,
            windows_job_name TEXT,
            printer_submitted_at TEXT,
            job_seen_at TEXT,
            status_detail TEXT,
            submitted_ip TEXT,
            queued_at TEXT,
            reviewed_at TEXT,
            reviewed_by INTEGER,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS global_config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER,
            action TEXT NOT NULL,
            details TEXT,
            ip TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token)",
        "CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_status_id ON print_tasks(status, id)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_user_status ON print_tasks(user_id, status)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_completed ON print_tasks(completed_at)",
        "CREATE INDEX IF NOT EXISTS idx_audit_created ON audit_log(created_at)",
        "CREATE INDEX IF NOT EXISTS idx_audit_user_created ON audit_log(user_id, created_at)",
        "INSERT OR IGNORE INTO global_config (key, value) VALUES ('queue_paused', 'false')",
    ];

    for statement in statements {
        pool.execute(statement).await?;
    }

    add_column_if_missing(pool, "print_tasks", "windows_job_id", "INTEGER").await?;
    add_column_if_missing(pool, "print_tasks", "windows_job_name", "TEXT").await?;
    add_column_if_missing(pool, "print_tasks", "printer_submitted_at", "TEXT").await?;
    add_column_if_missing(pool, "print_tasks", "job_seen_at", "TEXT").await?;
    add_column_if_missing(pool, "print_tasks", "status_detail", "TEXT").await?;
    add_column_if_missing(pool, "print_tasks", "submitted_ip", "TEXT").await?;
    add_column_if_missing(pool, "print_tasks", "queued_at", "TEXT").await?;
    add_column_if_missing(pool, "print_tasks", "reviewed_at", "TEXT").await?;
    add_column_if_missing(pool, "print_tasks", "reviewed_by", "INTEGER").await?;
    add_column_if_missing(pool, "print_tasks", "source_page_count", "INTEGER").await?;
    add_column_if_missing(pool, "print_tasks", "content_hash", "TEXT").await?;
    add_column_if_missing(
        pool,
        "temp_uploads",
        "byte_size",
        "INTEGER NOT NULL DEFAULT 0",
    )
    .await?;
    add_column_if_missing(
        pool,
        "temp_uploads",
        "content_hash",
        "TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    pool.execute(
        "CREATE INDEX IF NOT EXISTS idx_tasks_content_hash_submitted ON print_tasks(content_hash, submitted_at)",
    )
    .await?;
    add_column_if_missing(pool, "users", "last_login_at", "TEXT").await?;
    add_column_if_missing(pool, "users", "phone", "TEXT").await?;
    add_column_if_missing(pool, "users", "status", "TEXT NOT NULL DEFAULT 'unused'").await?;
    sqlx::query(
        "UPDATE users SET status = 'normal' WHERE status = 'unused' AND last_login_at IS NOT NULL",
    )
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM global_config WHERE key IN ('admin_qq', 'admin_student_id')")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS daily_usage")
        .execute(pool)
        .await?;

    Ok(())
}

async fn add_column_if_missing(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    definition: &str,
) -> AppResult<()> {
    let columns: Vec<String> =
        sqlx::query_scalar(&format!("SELECT name FROM pragma_table_info('{table}')"))
            .fetch_all(pool)
            .await?;
    if !columns.iter().any(|name| name == column) {
        pool.execute(format!("ALTER TABLE {table} ADD COLUMN {column} {definition}").as_str())
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn migration_is_idempotent_and_adds_current_columns() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        run(&pool).await.unwrap();
        run(&pool).await.unwrap();
        let columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('print_tasks')")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(columns.iter().any(|column| column == "windows_job_id"));
        assert!(columns.iter().any(|column| column == "status_detail"));
        assert!(columns.iter().any(|column| column == "source_page_count"));
        assert!(columns.iter().any(|column| column == "content_hash"));

        let user_columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('users')")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(user_columns.iter().any(|column| column == "phone"));
        assert!(user_columns.iter().any(|column| column == "status"));

        let upload_columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('temp_uploads')")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(upload_columns.iter().any(|column| column == "content_hash"));
    }

    #[tokio::test]
    async fn migration_preserves_unused_users_and_marks_previous_logins_normal() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            r#"
            CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                student_id TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'user',
                qq TEXT,
                must_change_password INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                last_login_at TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO users (student_id, password_hash, last_login_at) VALUES ('new', 'hash', NULL), ('used', 'hash', datetime('now'))",
        )
        .execute(&pool)
        .await
        .unwrap();

        run(&pool).await.unwrap();

        let statuses: Vec<(String, String)> =
            sqlx::query_as("SELECT student_id, status FROM users ORDER BY student_id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(
            statuses,
            vec![
                ("new".to_string(), "unused".to_string()),
                ("used".to_string(), "normal".to_string())
            ]
        );
    }
}
