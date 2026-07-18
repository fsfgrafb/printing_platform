use std::path::PathBuf;

use chrono::{Duration as ChronoDuration, Utc};
use tokio::{
    fs,
    time::{interval, Duration},
};
use tracing::{info, warn};

use crate::{app::AppState, error::AppResult};

#[derive(Debug, sqlx::FromRow)]
struct FileRow {
    id: i64,
    stored_path: Option<String>,
    preview_path: Option<String>,
}

pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        let hours = state.config.cleanup_interval_hours.max(1);
        let mut tick = interval(Duration::from_secs(hours * 60 * 60));

        loop {
            tick.tick().await;
            if let Err(error) = run_once(&state).await {
                warn!(?error, "cleanup task failed");
            }
        }
    });
}

async fn run_once(state: &AppState) -> AppResult<()> {
    let temp_cutoff = (Utc::now()
        - ChronoDuration::hours(state.config.temp_upload_retention_hours.max(1)))
    .format("%Y-%m-%d %H:%M:%S")
    .to_string();
    let history_cutoff = (Utc::now()
        - ChronoDuration::days(state.config.file_retention_days.max(1)))
    .format("%Y-%m-%d %H:%M:%S")
    .to_string();

    let temp_uploads = sqlx::query_as::<_, FileRow>(
        r#"
        SELECT id, stored_path, preview_path
        FROM temp_uploads
        WHERE created_at < ?
        "#,
    )
    .bind(&temp_cutoff)
    .fetch_all(&state.pool)
    .await?;

    let mut removed_uploads = 0;
    for upload in &temp_uploads {
        if remove_paths(upload).await {
            sqlx::query("DELETE FROM temp_uploads WHERE id = ?")
                .bind(upload.id)
                .execute(&state.pool)
                .await?;
            removed_uploads += 1;
        }
    }

    let old_tasks = sqlx::query_as::<_, FileRow>(
        r#"
        SELECT id, stored_path, preview_path
        FROM print_tasks
        WHERE status IN ('done', 'cancelled')
          AND COALESCE(completed_at, submitted_at) < ?
        "#,
    )
    .bind(&history_cutoff)
    .fetch_all(&state.pool)
    .await?;

    let mut removed_tasks = 0;
    for task in &old_tasks {
        if remove_paths(task).await {
            sqlx::query("DELETE FROM print_tasks WHERE id = ?")
                .bind(task.id)
                .execute(&state.pool)
                .await?;
            removed_tasks += 1;
        }
    }
    sqlx::query("DELETE FROM sessions WHERE expires_at <= ?")
        .bind(Utc::now().to_rfc3339())
        .execute(&state.pool)
        .await?;

    if !temp_uploads.is_empty() || !old_tasks.is_empty() {
        info!(
            temp_uploads = removed_uploads,
            old_tasks = removed_tasks,
            "cleanup removed expired records"
        );
    }

    Ok(())
}

async fn remove_paths(row: &FileRow) -> bool {
    let mut ok = true;
    for path in [&row.stored_path, &row.preview_path]
        .into_iter()
        .flatten()
        .map(PathBuf::from)
    {
        if let Err(error) = fs::remove_file(&path).await {
            if error.kind() != std::io::ErrorKind::NotFound {
                warn!(?error, row_id = row.id, path = %path.display(), "failed to remove expired file");
                ok = false;
            }
        }
    }
    ok
}
