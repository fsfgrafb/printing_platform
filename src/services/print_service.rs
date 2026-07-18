use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use lopdf::Document;
use sqlx::SqlitePool;
use tokio::task;
use uuid::Uuid;

use crate::{
    db::models::{PrintTask, User},
    error::{AppError, AppResult},
};

const TASK_COLUMNS: &str = "id, user_id, file_name, stored_path, preview_path, page_count, odd_even, status, submitted_at, completed_at, cancelled_by, review_reason, approved_over_quota, windows_job_id, windows_job_name, printer_submitted_at, job_seen_at, status_detail";

pub fn selected_page_count(total: i64, odd_even: &str) -> AppResult<i64> {
    let total = total.max(1);
    let selected = match odd_even {
        "all" => total,
        "odd" => (total + 1) / 2,
        "even" => total / 2,
        selection if selection.starts_with("custom:") => {
            let total = u32::try_from(total)
                .map_err(|_| AppError::BadRequest("文件页数超出支持范围".to_string()))?;
            i64::try_from(parse_custom_pages(total, &selection["custom:".len()..])?.len())
                .map_err(|_| AppError::BadRequest("自定义页码数量超出支持范围".to_string()))?
        }
        _ => {
            return Err(AppError::BadRequest(
                "页码范围必须是全部页、奇数页、偶数页或自定义页码".to_string(),
            ))
        }
    };

    if selected == 0 {
        Err(AppError::BadRequest("所选页码中没有可打印页面".to_string()))
    } else {
        Ok(selected)
    }
}

pub fn normalize_custom_page_range(total: i64, input: &str) -> AppResult<String> {
    let total = u32::try_from(total.max(1))
        .map_err(|_| AppError::BadRequest("文件页数超出支持范围".to_string()))?;
    let pages = parse_custom_pages(total, input)?;
    Ok(format!("custom:{}", format_page_ranges(&pages)))
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
        selection if selection.starts_with("custom:") => {
            let selected = parse_custom_pages(total_pages, &selection["custom:".len()..])?;
            (1..=total_pages)
                .filter(|page| !selected.contains(page))
                .collect()
        }
        _ => {
            return Err(AppError::BadRequest(
                "页码范围必须是全部页、奇数页、偶数页或自定义页码".to_string(),
            ))
        }
    };

    if delete_pages.len() == total_pages as usize {
        Err(AppError::BadRequest("所选页码中没有可打印页面".to_string()))
    } else {
        Ok(delete_pages)
    }
}

fn parse_custom_pages(total_pages: u32, input: &str) -> AppResult<BTreeSet<u32>> {
    let mut pages = BTreeSet::new();
    for token in input
        .split(|character: char| {
            character.is_whitespace() || matches!(character, ',' | '，' | '、')
        })
        .filter(|token| !token.is_empty())
    {
        let mut bounds = token.split('-');
        let start = parse_page_number(bounds.next().unwrap_or_default(), total_pages)?;
        let end = match bounds.next() {
            Some(value) => parse_page_number(value, total_pages)?,
            None => start,
        };
        if bounds.next().is_some() {
            return Err(AppError::BadRequest(format!(
                "页码格式“{token}”无效，请使用数字或起止页（如 2-5）"
            )));
        }
        if start > end {
            return Err(AppError::BadRequest(format!(
                "页码范围“{token}”的起始页不能大于结束页"
            )));
        }
        for page in start..=end {
            if !pages.insert(page) {
                return Err(AppError::BadRequest(format!(
                    "自定义页码中重复指定了第 {page} 页"
                )));
            }
        }
    }
    if pages.is_empty() {
        return Err(AppError::BadRequest("请输入要打印的自定义页码".to_string()));
    }
    Ok(pages)
}

fn parse_page_number(value: &str, total_pages: u32) -> AppResult<u32> {
    let page = value.parse::<u32>().map_err(|_| {
        AppError::BadRequest(format!("页码“{value}”无效，请使用数字或起止页（如 2-5）"))
    })?;
    if page == 0 || page > total_pages {
        return Err(AppError::BadRequest(format!(
            "页码 {page} 超出有效范围 1-{total_pages}"
        )));
    }
    Ok(page)
}

fn format_page_ranges(pages: &BTreeSet<u32>) -> String {
    let mut ranges = Vec::new();
    let mut iter = pages.iter().copied();
    let Some(mut start) = iter.next() else {
        return String::new();
    };
    let mut end = start;
    for page in iter {
        if page == end + 1 {
            end = page;
            continue;
        }
        ranges.push(format_page_range(start, end));
        start = page;
        end = page;
    }
    ranges.push(format_page_range(start, end));
    ranges.join(",")
}

fn format_page_range(start: u32, end: u32) -> String {
    if start == end {
        start.to_string()
    } else {
        format!("{start}-{end}")
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
    let task = load_task(pool, task_id).await?;

    if !actor.is_admin() && task.user_id != actor.id {
        return Err(AppError::Forbidden);
    }

    if task.status != "queued"
        && task.status != "pending_review"
        && !(actor.is_admin() && task.status == "uncertain")
    {
        return Err(AppError::Conflict("该任务当前不能安全取消".to_string()));
    }

    let cancelled_by = if actor.is_admin() { "admin" } else { "user" };
    sqlx::query(
        r#"
        UPDATE print_tasks
        SET status = 'cancelled', cancelled_by = ?, review_reason = ?,
            completed_at = datetime('now'), status_detail = '任务已取消'
        WHERE id = ? AND status = ?
        "#,
    )
    .bind(cancelled_by)
    .bind(reason)
    .bind(task_id)
    .bind(&task.status)
    .execute(pool)
    .await?;

    load_task(pool, task_id).await
}

pub async fn load_task(pool: &SqlitePool, task_id: i64) -> AppResult<PrintTask> {
    let query = format!("SELECT {TASK_COLUMNS} FROM print_tasks WHERE id = ?");
    sqlx::query_as::<_, PrintTask>(&query)
        .bind(task_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("task not found".to_string()))
}

#[cfg(test)]
mod tests {
    use super::{normalize_custom_page_range, pages_to_delete, quota_status, selected_page_count};

    #[test]
    fn selected_page_count_handles_page_ranges() {
        assert_eq!(selected_page_count(5, "all").unwrap(), 5);
        assert_eq!(selected_page_count(5, "odd").unwrap(), 3);
        assert_eq!(selected_page_count(5, "even").unwrap(), 2);
        assert_eq!(selected_page_count(8, "custom:1-3,5，7、8").unwrap(), 6);
        assert!(selected_page_count(1, "even").is_err());
        assert!(selected_page_count(5, "invalid").is_err());
    }

    #[test]
    fn custom_page_ranges_are_validated_and_normalized() {
        assert_eq!(
            normalize_custom_page_range(10, "1-3，5、7 8").unwrap(),
            "custom:1-3,5,7-8"
        );
        assert!(normalize_custom_page_range(10, "").is_err());
        assert!(normalize_custom_page_range(10, "0").is_err());
        assert!(normalize_custom_page_range(10, "11").is_err());
        assert!(normalize_custom_page_range(10, "4-2").is_err());
        assert!(normalize_custom_page_range(10, "1-3,3").is_err());
        assert!(normalize_custom_page_range(10, "1--3").is_err());
    }

    #[test]
    fn pages_to_delete_matches_selected_range() {
        assert_eq!(pages_to_delete(5, "all").unwrap(), Vec::<u32>::new());
        assert_eq!(pages_to_delete(5, "odd").unwrap(), vec![2, 4]);
        assert_eq!(pages_to_delete(5, "even").unwrap(), vec![1, 3, 5]);
        assert_eq!(pages_to_delete(8, "custom:1-3,5,7-8").unwrap(), vec![4, 6]);
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
