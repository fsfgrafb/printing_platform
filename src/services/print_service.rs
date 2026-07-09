use std::path::{Path, PathBuf};

use lopdf::Document;
use sqlx::SqlitePool;
use tokio::task;
use uuid::Uuid;

use crate::{
    db::models::{PrintTask, User},
    error::{AppError, AppResult},
};

pub fn selected_page_count(total: i64, odd_even: &str) -> AppResult<i64> {
    let total = total.max(1);
    let selected = match odd_even {
        "all" => total,
        "odd" => (total + 1) / 2,
        "even" => total / 2,
        _ => Err(AppError::BadRequest(
            "odd_even must be one of all, odd, even".to_string(),
        ))?,
    };

    if selected == 0 {
        Err(AppError::BadRequest(
            "selected page range contains no pages".to_string(),
        ))
    } else {
        Ok(selected)
    }
}

pub async fn apply_page_selection(pdf_path: &Path, odd_even: &str) -> AppResult<()> {
    if odd_even == "all" {
        return Ok(());
    }

    let pdf_path = pdf_path.to_path_buf();
    let odd_even = odd_even.to_string();
    task::spawn_blocking(move || write_selected_pages(&pdf_path, &odd_even))
        .await
        .map_err(|error| AppError::External(format!("page selection task failed: {error}")))?
}

pub fn quota_status(projected_used: &mut i64, page_count: i64, limit: i64) -> (&'static str, bool) {
    if *projected_used + page_count > limit {
        ("pending_review", true)
    } else {
        *projected_used += page_count;
        ("queued", false)
    }
}

fn write_selected_pages(pdf_path: &Path, odd_even: &str) -> AppResult<()> {
    let mut document = Document::load(pdf_path)
        .map_err(|error| AppError::External(format!("failed to read preview PDF: {error}")))?;
    let delete_pages = pages_to_delete(document.get_pages().len() as u32, odd_even)?;
    if delete_pages.is_empty() {
        return Ok(());
    }

    document.delete_pages(&delete_pages);
    document.prune_objects();
    document.renumber_objects();
    document.compress();

    let temp_path = temporary_pdf_path(pdf_path);
    document.save(&temp_path)?;
    std::fs::copy(&temp_path, pdf_path)?;
    let _ = std::fs::remove_file(temp_path);

    Ok(())
}

fn pages_to_delete(total_pages: u32, odd_even: &str) -> AppResult<Vec<u32>> {
    if total_pages == 0 {
        return Err(AppError::BadRequest("preview PDF has no pages".to_string()));
    }

    let delete_pages: Vec<u32> = match odd_even {
        "all" => Vec::new(),
        "odd" => (1..=total_pages).filter(|page| page % 2 == 0).collect(),
        "even" => (1..=total_pages).filter(|page| page % 2 == 1).collect(),
        _ => {
            return Err(AppError::BadRequest(
                "odd_even must be one of all, odd, even".to_string(),
            ))
        }
    };

    if delete_pages.len() == total_pages as usize {
        Err(AppError::BadRequest(
            "selected page range contains no pages".to_string(),
        ))
    } else {
        Ok(delete_pages)
    }
}

fn temporary_pdf_path(pdf_path: &Path) -> PathBuf {
    let file_name = pdf_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("preview.pdf");
    pdf_path.with_file_name(format!("{file_name}.{}.tmp", Uuid::new_v4()))
}

pub async fn cancel_task(
    pool: &SqlitePool,
    task_id: i64,
    actor: &User,
    reason: Option<String>,
) -> AppResult<PrintTask> {
    let task = sqlx::query_as::<_, PrintTask>(
        r#"
        SELECT id, user_id, file_name, stored_path, preview_path, page_count, odd_even,
               status, submitted_at, completed_at, cancelled_by, review_reason, approved_over_quota
        FROM print_tasks
        WHERE id = ?
        "#,
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("task not found".to_string()))?;

    if !actor.is_admin() && task.user_id != actor.id {
        return Err(AppError::Forbidden);
    }

    if task.status != "queued" && task.status != "pending_review" {
        return Err(AppError::Conflict(
            "only queued or pending review tasks can be cancelled".to_string(),
        ));
    }

    let cancelled_by = if actor.is_admin() { "admin" } else { "user" };
    sqlx::query(
        r#"
        UPDATE print_tasks
        SET status = 'cancelled', cancelled_by = ?, review_reason = ?
        WHERE id = ?
        "#,
    )
    .bind(cancelled_by)
    .bind(reason)
    .bind(task_id)
    .execute(pool)
    .await?;

    let updated = sqlx::query_as::<_, PrintTask>(
        r#"
        SELECT id, user_id, file_name, stored_path, preview_path, page_count, odd_even,
               status, submitted_at, completed_at, cancelled_by, review_reason, approved_over_quota
        FROM print_tasks
        WHERE id = ?
        "#,
    )
    .bind(task_id)
    .fetch_one(pool)
    .await?;

    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::{pages_to_delete, quota_status, selected_page_count};

    #[test]
    fn selected_page_count_handles_page_ranges() {
        assert_eq!(selected_page_count(5, "all").unwrap(), 5);
        assert_eq!(selected_page_count(5, "odd").unwrap(), 3);
        assert_eq!(selected_page_count(5, "even").unwrap(), 2);
        assert!(selected_page_count(1, "even").is_err());
        assert!(selected_page_count(5, "invalid").is_err());
    }

    #[test]
    fn pages_to_delete_matches_selected_range() {
        assert_eq!(pages_to_delete(5, "all").unwrap(), Vec::<u32>::new());
        assert_eq!(pages_to_delete(5, "odd").unwrap(), vec![2, 4]);
        assert_eq!(pages_to_delete(5, "even").unwrap(), vec![1, 3, 5]);
        assert!(pages_to_delete(1, "even").is_err());
    }

    #[test]
    fn quota_status_accumulates_queued_pages_in_one_submission() {
        let mut projected_used = 40;

        assert_eq!(quota_status(&mut projected_used, 5, 50), ("queued", false));
        assert_eq!(projected_used, 45);

        assert_eq!(quota_status(&mut projected_used, 5, 50), ("queued", false));
        assert_eq!(projected_used, 50);

        assert_eq!(
            quota_status(&mut projected_used, 1, 50),
            ("pending_review", true)
        );
        assert_eq!(projected_used, 50);
    }

    #[test]
    fn quota_status_keeps_remaining_capacity_after_review_task() {
        let mut projected_used = 45;

        assert_eq!(
            quota_status(&mut projected_used, 10, 50),
            ("pending_review", true)
        );
        assert_eq!(projected_used, 45);

        assert_eq!(quota_status(&mut projected_used, 3, 50), ("queued", false));
        assert_eq!(projected_used, 48);
    }
}
