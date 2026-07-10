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

const TASK_COLUMNS: &str = "id, user_id, file_name, stored_path, preview_path, page_count, odd_even, status, submitted_at, completed_at, cancelled_by, review_reason, approved_over_quota, windows_job_id, windows_job_name, printer_submitted_at, job_seen_at, status_detail";

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
    let _queue_guard = state.queue_lock.lock().await;
    refresh_printer_state(state).await;
    let snapshot = state.printer_state.read().await.clone();

    if let Some(task) = printing_task(&state.pool).await? {
        track_printing_task(state, &snapshot, task).await?;
        return Ok(());
    }

    if settings::queue_paused(&state.pool).await? || snapshot.blocked {
        return Ok(());
    }

    let Some(task) = next_task(&state.pool).await? else {
        return Ok(());
    };
    let Some(preview_path) = task.preview_path.clone() else {
        fail_task(state, &task, "任务没有可打印的 PDF 文件").await?;
        return Ok(());
    };

    if !mark_printing(&state.pool, task.id).await? {
        return Ok(());
    }
    state.broadcaster.send(QueueEvent::TaskStatus {
        task_id: task.id,
        status: "printing".into(),
    });
    state.broadcaster.send(QueueEvent::QueueChanged);

    info!(
        task_id = task.id,
        path = preview_path,
        "submitting task to Windows printer queue"
    );
    match printer::submit_pdf(&state.config, &PathBuf::from(&preview_path), task.id).await {
        Ok(Some(job)) => {
            sqlx::query(
                "UPDATE print_tasks SET windows_job_id = ?, windows_job_name = ?, printer_submitted_at = datetime('now'), job_seen_at = datetime('now'), status_detail = '已提交至 Windows 打印队列' WHERE id = ?",
            )
            .bind(job.job_id).bind(job.job_name).bind(task.id).execute(&state.pool).await?;
        }
        Ok(None) => finish_task(&state.pool, &task).await?,
        Err(error) => submission_failed(state, &task, &error.to_string()).await?,
    }

    state.broadcaster.send(QueueEvent::QueueChanged);
    Ok(())
}

async fn refresh_printer_state(state: &AppState) {
    let mut next = printer::query_status(&state.config).await;
    let mut current = state.printer_state.write().await;
    let had_toner_warning = !current.warnings.is_empty();
    if !next.warnings.is_empty() && had_toner_warning {
        next.toner_alert_acknowledged = current.toner_alert_acknowledged;
    }
    let materially_changed = current.status != next.status
        || current.blocked != next.blocked
        || current.blocking_reasons != next.blocking_reasons
        || current.warnings != next.warnings
        || current.error != next.error;
    *current = next.clone();
    drop(current);
    if materially_changed {
        state
            .broadcaster
            .send(QueueEvent::PrinterStatus { printer: next });
    }
}

async fn track_printing_task(
    state: &AppState,
    snapshot: &printer::PrinterState,
    task: PrintTask,
) -> AppResult<()> {
    if state.config.printer.simulate {
        finish_task(&state.pool, &task).await?;
        state.broadcaster.send(QueueEvent::TaskStatus {
            task_id: task.id,
            status: "done".into(),
        });
        state.broadcaster.send(QueueEvent::QueueChanged);
        return Ok(());
    }

    if !snapshot.available || snapshot.error.is_some() {
        return Ok(());
    }

    let matching_job = task
        .windows_job_id
        .and_then(|id| {
            snapshot.jobs.iter().find(|job| {
                job.id == id
                    && task
                        .windows_job_name
                        .as_ref()
                        .map(|name| name == &job.name)
                        .unwrap_or(true)
            })
        })
        .or_else(|| {
            task.windows_job_name
                .as_deref()
                .and_then(|name| snapshot.jobs.iter().find(|job| job.name == name))
        })
        .or_else(|| {
            let marker = format!("print-task-{}", task.id).to_ascii_lowercase();
            snapshot
                .jobs
                .iter()
                .find(|job| job.name.to_ascii_lowercase().contains(&marker))
        });
    if let Some(job) = matching_job {
        sqlx::query("UPDATE print_tasks SET windows_job_id = ?, windows_job_name = ?, job_seen_at = datetime('now'), status_detail = ? WHERE id = ?")
            .bind(job.id).bind(&job.name).bind(format!("Windows 作业处理中：{}", job.status)).bind(task.id)
            .execute(&state.pool).await?;
        return Ok(());
    }

    if task.job_seen_at.is_some() {
        finish_task(&state.pool, &task).await?;
        state.broadcaster.send(QueueEvent::TaskStatus {
            task_id: task.id,
            status: "done".into(),
        });
        state.broadcaster.send(QueueEvent::QueueChanged);
        return Ok(());
    }

    // A crash may occur between launching the shell print handler and storing its job id.
    // Keep the task recoverable, but do not silently submit it twice.
    settings::set_queue_paused(&state.pool, true).await?;
    sqlx::query("UPDATE print_tasks SET status = 'queued', status_detail = '未能确认 Windows 作业，已暂停以避免重复打印' WHERE id = ?")
        .bind(task.id).execute(&state.pool).await?;
    state.broadcaster.send(QueueEvent::PrinterError {
        message: "未能确认 Windows 打印作业；请管理员检查打印队列后再继续".into(),
    });
    state
        .broadcaster
        .send(QueueEvent::QueuePaused { paused: true });
    Ok(())
}

async fn next_task(pool: &SqlitePool) -> AppResult<Option<PrintTask>> {
    let query = format!(
        "SELECT {TASK_COLUMNS} FROM print_tasks WHERE status = 'queued' ORDER BY submitted_at ASC, id ASC LIMIT 1"
    );
    Ok(sqlx::query_as::<_, PrintTask>(&query)
        .fetch_optional(pool)
        .await?)
}

async fn printing_task(pool: &SqlitePool) -> AppResult<Option<PrintTask>> {
    let query = format!(
        "SELECT {TASK_COLUMNS} FROM print_tasks WHERE status = 'printing' ORDER BY id ASC LIMIT 1"
    );
    Ok(sqlx::query_as::<_, PrintTask>(&query)
        .fetch_optional(pool)
        .await?)
}

async fn mark_printing(pool: &SqlitePool, task_id: i64) -> AppResult<bool> {
    let affected = sqlx::query("UPDATE print_tasks SET status = 'printing', status_detail = '正在提交至打印机' WHERE id = ? AND status = 'queued'")
        .bind(task_id).execute(pool).await?.rows_affected();
    Ok(affected == 1)
}

async fn finish_task(pool: &SqlitePool, task: &PrintTask) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    let affected = sqlx::query("UPDATE print_tasks SET status = 'done', completed_at = datetime('now'), status_detail = 'Windows 打印作业已结束' WHERE id = ? AND status = 'printing'")
        .bind(task.id).execute(&mut *tx).await?.rows_affected();
    if affected > 0 && !task.approved_over_quota {
        quota::add_usage_tx(&mut tx, task.user_id, task.page_count).await?;
    }
    tx.commit().await?;
    remove_spool_copy(task).await;
    Ok(())
}

async fn remove_spool_copy(task: &PrintTask) {
    let Some(preview) = task.preview_path.as_deref() else {
        return;
    };
    let Some(parent) = std::path::Path::new(preview).parent() else {
        return;
    };
    let _ = tokio::fs::remove_file(parent.join(format!("print-task-{}.pdf", task.id))).await;
}

async fn submission_failed(state: &AppState, task: &PrintTask, reason: &str) -> AppResult<()> {
    error!(
        task_id = task.id,
        reason, "printer submission failed; pausing queue"
    );
    settings::set_queue_paused(&state.pool, true).await?;
    sqlx::query("UPDATE print_tasks SET status = 'queued', status_detail = ?, review_reason = ? WHERE id = ?")
        .bind(format!("提交失败：{reason}")).bind(reason).bind(task.id).execute(&state.pool).await?;
    state.broadcaster.send(QueueEvent::PrinterError {
        message: reason.into(),
    });
    state
        .broadcaster
        .send(QueueEvent::QueuePaused { paused: true });
    Ok(())
}

async fn fail_task(state: &AppState, task: &PrintTask, reason: &str) -> AppResult<()> {
    sqlx::query("UPDATE print_tasks SET status = 'cancelled', cancelled_by = 'system', review_reason = ?, completed_at = datetime('now'), status_detail = ? WHERE id = ?")
        .bind(reason).bind(reason).bind(task.id).execute(&state.pool).await?;
    state.broadcaster.send(QueueEvent::TaskStatus {
        task_id: task.id,
        status: "cancelled".into(),
    });
    Err(AppError::External(reason.into()))
}
