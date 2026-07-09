use std::path::PathBuf;

use sqlx::SqlitePool;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use crate::{
    app::AppState,
    db::models::PrintTask,
    error::{AppError, AppResult},
    services::{printer, quota, settings},
    ws::QueueEvent,
};

pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(state.config.queue_poll_seconds.max(1)));
        loop {
            tick.tick().await;
            if let Err(error) = run_once(&state).await {
                warn!(?error, "queue manager tick failed");
            }
        }
    });
}

async fn run_once(state: &AppState) -> AppResult<()> {
    if settings::queue_paused(&state.pool).await? {
        return Ok(());
    }

    let Some(task) = next_task(&state.pool).await? else {
        return Ok(());
    };

    mark_printing(&state.pool, task.id).await?;
    state.broadcaster.send(QueueEvent::TaskStatus {
        task_id: task.id,
        status: "printing".to_string(),
    });
    state.broadcaster.send(QueueEvent::QueueChanged);

    let Some(preview_path) = task.preview_path.clone() else {
        fail_task(state, &task, "task has no preview PDF").await?;
        return Ok(());
    };

    info!(
        task_id = task.id,
        path = preview_path,
        "submitting task to printer"
    );
    match printer::print_pdf(&state.config, &PathBuf::from(&preview_path)).await {
        Ok(()) => finish_task(&state.pool, &task).await?,
        Err(error) => {
            error!(?error, task_id = task.id, "printing failed; pausing queue");
            settings::set_queue_paused(&state.pool, true).await?;
            sqlx::query(
                r#"
                UPDATE print_tasks
                SET status = 'queued', review_reason = ?
                WHERE id = ?
                "#,
            )
            .bind(error.to_string())
            .bind(task.id)
            .execute(&state.pool)
            .await?;
            state.broadcaster.send(QueueEvent::PrinterError {
                message: error.to_string(),
            });
            state
                .broadcaster
                .send(QueueEvent::QueuePaused { paused: true });
        }
    }

    state.broadcaster.send(QueueEvent::QueueChanged);
    Ok(())
}

async fn next_task(pool: &SqlitePool) -> AppResult<Option<PrintTask>> {
    Ok(sqlx::query_as::<_, PrintTask>(
        r#"
        SELECT id, user_id, file_name, stored_path, preview_path, page_count, odd_even,
               status, submitted_at, completed_at, cancelled_by, review_reason, approved_over_quota
        FROM print_tasks
        WHERE status = 'queued'
        ORDER BY id ASC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await?)
}

async fn mark_printing(pool: &SqlitePool, task_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE print_tasks SET status = 'printing' WHERE id = ?")
        .bind(task_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn finish_task(pool: &SqlitePool, task: &PrintTask) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE print_tasks
        SET status = 'done', completed_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(task.id)
    .execute(pool)
    .await?;
    quota::add_usage(pool, task.user_id, task.page_count).await?;
    Ok(())
}

async fn fail_task(state: &AppState, task: &PrintTask, reason: &str) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE print_tasks
        SET status = 'cancelled', cancelled_by = 'system', review_reason = ?
        WHERE id = ?
        "#,
    )
    .bind(reason)
    .bind(task.id)
    .execute(&state.pool)
    .await?;
    state.broadcaster.send(QueueEvent::TaskStatus {
        task_id: task.id,
        status: "cancelled".to_string(),
    });
    Err(AppError::External(reason.to_string()))
}
