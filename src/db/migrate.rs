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
            must_change_password INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
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
            odd_even TEXT NOT NULL DEFAULT 'all',
            status TEXT NOT NULL DEFAULT 'queued',
            submitted_at TEXT NOT NULL DEFAULT (datetime('now')),
            completed_at TEXT,
            cancelled_by TEXT,
            review_reason TEXT,
            approved_over_quota INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS daily_usage (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            date TEXT NOT NULL,
            page_count INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            UNIQUE(user_id, date)
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
        "CREATE INDEX IF NOT EXISTS idx_tasks_status_id ON print_tasks(status, id)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_user_status ON print_tasks(user_id, status)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_completed ON print_tasks(completed_at)",
        "INSERT OR IGNORE INTO global_config (key, value) VALUES ('daily_limit', '50')",
        "INSERT OR IGNORE INTO global_config (key, value) VALUES ('admin_qq', '')",
        "INSERT OR IGNORE INTO global_config (key, value) VALUES ('admin_student_id', '')",
        "INSERT OR IGNORE INTO global_config (key, value) VALUES ('queue_paused', 'false')",
    ];

    for statement in statements {
        pool.execute(statement).await?;
    }

    Ok(())
}
