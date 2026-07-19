use std::{collections::HashSet, net::SocketAddr, path::PathBuf};

use axum::{
    body::Body,
    extract::{ConnectInfo, Multipart, Path, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, Request},
    response::Response,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncWriteExt};
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    app::AppState,
    auth::middleware::CurrentUser,
    db::models::{PrintTask, TempUpload},
    error::{AppError, AppResult},
    services::{audit, converter, print_service, quota},
    utils,
    ws::QueueEvent,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/print/uploads", get(list_uploads))
        .route("/print/upload", post(upload))
        .route("/print/uploads/:temp_id", delete(remove_upload))
        .route("/print/preview/:temp_id", get(preview))
        .route("/print/tasks/:task_id/preview", get(task_preview))
        .route("/print/tasks/:task_id/source", get(task_source))
        .route("/print/submit", post(submit))
        .route("/print/tasks/:task_id", delete(cancel))
}

pub async fn list_uploads(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<UploadResponse>> {
    let uploads = sqlx::query_as::<_, TempUpload>(
        r#"
        SELECT id, temp_id, user_id, original_name, stored_path, preview_path, page_count, byte_size, created_at
        FROM temp_uploads
        WHERE user_id = ?
        ORDER BY created_at ASC, id ASC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(UploadResponse {
        files: uploads
            .into_iter()
            .map(|upload| UploadItem {
                preview_url: format!("/api/print/preview/{}", upload.temp_id),
                temp_id: upload.temp_id,
                original_name: upload.original_name,
                page_count: upload.page_count,
            })
            .collect(),
    }))
}

#[derive(Debug, Serialize)]
pub struct UploadItem {
    pub temp_id: String,
    pub original_name: String,
    pub page_count: i64,
    pub preview_url: String,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub files: Vec<UploadItem>,
}

pub async fn upload(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    mut multipart: Multipart,
) -> AppResult<Json<UploadResponse>> {
    let mut files = Vec::new();
    let existing_bytes: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(byte_size), 0) FROM temp_uploads WHERE user_id = ?",
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;
    let mut user_bytes = u64::try_from(existing_bytes).unwrap_or(0);

    while let Some(mut field) = multipart.next_field().await? {
        if field.file_name().is_none() {
            continue;
        }

        let original_name = field.file_name().unwrap_or("upload.bin").to_string();
        converter::ensure_supported_file_name(&original_name)?;
        let temp_id = Uuid::new_v4().to_string();
        if files.len() >= state.config.limits.max_files_per_request {
            return Err(AppError::BadRequest("单次上传文件数量超出限制".into()));
        }
        let stored_path = upload_path(&state.config, &original_name);
        let preview_path = utils::preview_dir(&state.config).join(format!("{temp_id}.pdf"));

        info!(%temp_id, file_name = %original_name, "receiving uploaded file");
        let mut output = fs::File::create(&stored_path).await?;
        let mut byte_size = 0_u64;
        while let Some(chunk) = field.chunk().await? {
            byte_size = byte_size.saturating_add(chunk.len() as u64);
            if byte_size > state.config.limits.max_upload_bytes
                || user_bytes.saturating_add(byte_size)
                    > state.config.limits.max_temp_bytes_per_user
            {
                drop(output);
                let _ = fs::remove_file(&stored_path).await;
                return Err(AppError::BadRequest(
                    "文件过大或当前用户的临时文件空间已满".into(),
                ));
            }
            output.write_all(&chunk).await?;
        }
        output.flush().await?;
        drop(output);
        info!(%temp_id, path = %stored_path.display(), "uploaded file saved");

        let conversion_slot = state
            .conversion_slots
            .acquire()
            .await
            .map_err(|_| AppError::External("conversion worker stopped".into()))?;
        let page_count =
            match converter::prepare_preview(&state.config, &stored_path, &preview_path).await {
                Ok(count) => count,
                Err(error) => {
                    warn!(
                        %temp_id,
                        file_name = %original_name,
                        error = %error,
                        "failed to prepare uploaded file preview"
                    );
                    let _ = fs::remove_file(&stored_path).await;
                    let _ = fs::remove_file(&preview_path).await;
                    return Err(AppError::BadRequest(format!(
                        "无法转换文件“{original_name}”，请确认文件未损坏且 LibreOffice 支持该格式"
                    )));
                }
            };
        drop(conversion_slot);
        if page_count > state.config.limits.max_pages_per_file {
            let _ = fs::remove_file(&stored_path).await;
            let _ = fs::remove_file(&preview_path).await;
            return Err(AppError::BadRequest(format!(
                "文件页数超过 {} 页限制",
                state.config.limits.max_pages_per_file
            )));
        }
        info!(%temp_id, page_count, preview = %preview_path.display(), "preview ready");
        sqlx::query(
            r#"
            INSERT INTO temp_uploads (temp_id, user_id, original_name, stored_path, preview_path, page_count, byte_size)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&temp_id)
        .bind(user.id)
        .bind(&original_name)
        .bind(stored_path.to_string_lossy().to_string())
        .bind(preview_path.to_string_lossy().to_string())
        .bind(page_count)
        .bind(i64::try_from(byte_size).unwrap_or(i64::MAX))
        .execute(&state.pool)
        .await?;
        user_bytes = user_bytes.saturating_add(byte_size);

        files.push(UploadItem {
            temp_id: temp_id.clone(),
            original_name,
            page_count,
            preview_url: format!("/api/print/preview/{temp_id}"),
        });
    }

    Ok(Json(UploadResponse { files }))
}

pub async fn preview(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(temp_id): Path<String>,
) -> AppResult<Response> {
    let upload = sqlx::query_as::<_, TempUpload>(
        r#"
        SELECT id, temp_id, user_id, original_name, stored_path, preview_path, page_count, byte_size, created_at
        FROM temp_uploads
        WHERE temp_id = ?
        "#,
    )
    .bind(&temp_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("preview not found".to_string()))?;

    if !user.is_admin() && upload.user_id != user.id {
        return Err(AppError::Forbidden);
    }

    let path = PathBuf::from(upload.preview_path);
    if !path.exists() {
        return Err(AppError::NotFound("preview file not found".to_string()));
    }

    serve_file(path, Some("inline; filename=preview.pdf")).await
}

pub async fn task_preview(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(task_id): Path<i64>,
) -> AppResult<Response> {
    let (owner_id, preview_path): (i64, Option<String>) =
        sqlx::query_as("SELECT user_id, preview_path FROM print_tasks WHERE id = ?")
            .bind(task_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("print record not found".to_string()))?;

    if !user.is_admin() && owner_id != user.id {
        return Err(AppError::Forbidden);
    }

    let path = preview_path
        .map(PathBuf::from)
        .ok_or_else(|| AppError::NotFound("final PDF is not available".to_string()))?;
    serve_file(path, Some("inline; filename=print-preview.pdf")).await
}

pub async fn task_source(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(task_id): Path<i64>,
) -> AppResult<Response> {
    let (owner_id, stored_path, file_name): (i64, String, String) =
        sqlx::query_as("SELECT user_id, stored_path, file_name FROM print_tasks WHERE id = ?")
            .bind(task_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("print record not found".to_string()))?;

    if !user.is_admin() && owner_id != user.id {
        return Err(AppError::Forbidden);
    }

    let download_name = utils::sanitize_filename(&file_name);
    let disposition = format!("attachment; filename=\"{download_name}\"");
    serve_file(PathBuf::from(stored_path), Some(&disposition)).await
}

pub async fn remove_upload(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(temp_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let upload = sqlx::query_as::<_, TempUpload>(
        r#"
        SELECT id, temp_id, user_id, original_name, stored_path, preview_path, page_count, byte_size, created_at
        FROM temp_uploads
        WHERE temp_id = ?
        "#,
    )
    .bind(&temp_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("uploaded file not found".to_string()))?;

    if upload.user_id != user.id {
        return Err(AppError::Forbidden);
    }

    sqlx::query("DELETE FROM temp_uploads WHERE id = ?")
        .bind(upload.id)
        .execute(&state.pool)
        .await?;
    for path in [&upload.stored_path, &upload.preview_path] {
        if let Err(error) = fs::remove_file(path).await {
            if error.kind() != std::io::ErrorKind::NotFound {
                warn!(?error, %temp_id, %path, "failed to remove temporary upload file");
            }
        }
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct SubmitRequest {
    pub files: Vec<SubmitFile>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitFile {
    pub temp_id: String,
    #[serde(default = "default_odd_even")]
    pub odd_even: String,
    #[serde(default)]
    pub page_range: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SubmittedTask {
    pub id: i64,
    pub file_name: String,
    pub page_count: i64,
    pub odd_even: String,
    pub status: String,
    pub over_limit: bool,
}

#[derive(Debug, Serialize)]
pub struct SubmitResponse {
    pub tasks: Vec<SubmittedTask>,
    pub used_today: i64,
    pub limit: i64,
}

struct PreparedFile {
    upload: TempUpload,
    odd_even: String,
    page_count: i64,
    task_preview_path: PathBuf,
}

pub async fn submit(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    Json(request): Json<SubmitRequest>,
) -> AppResult<Json<SubmitResponse>> {
    if request.files.is_empty() {
        return Err(AppError::BadRequest("files cannot be empty".to_string()));
    }

    let mut temp_ids = HashSet::new();
    for file in &request.files {
        if !matches!(file.odd_even.as_str(), "all" | "odd" | "even" | "custom") {
            return Err(AppError::BadRequest(
                "页码范围必须是全部页、奇数页、偶数页或自定义页码".to_string(),
            ));
        }
        if file.odd_even == "custom"
            && file
                .page_range
                .as_deref()
                .is_none_or(|range| range.trim().is_empty())
        {
            return Err(AppError::BadRequest("请输入要打印的自定义页码".to_string()));
        }
        if !temp_ids.insert(file.temp_id.as_str()) {
            return Err(AppError::BadRequest("提交内容中包含重复文件".to_string()));
        }
    }

    let _submission_guard = state.submission_lock.lock().await;
    let used_today = quota::used_today(&state.pool, user.id).await?;
    let reserved = quota::reserved(&state.pool, user.id).await?;
    let limit = quota::daily_limit(&state.pool).await?;
    let mut prepared = Vec::with_capacity(request.files.len());

    for file in request.files {
        let upload_result = sqlx::query_as::<_, TempUpload>(
            r#"
            SELECT id, temp_id, user_id, original_name, stored_path, preview_path, page_count, byte_size, created_at
            FROM temp_uploads
            WHERE temp_id = ?
            "#,
        )
        .bind(&file.temp_id)
        .fetch_optional(&state.pool)
        .await;
        let upload = match upload_result {
            Ok(Some(upload)) => upload,
            Ok(None) => {
                cleanup_prepared(&prepared).await;
                return Err(AppError::NotFound("uploaded file not found".to_string()));
            }
            Err(error) => {
                cleanup_prepared(&prepared).await;
                return Err(error.into());
            }
        };

        if upload.user_id != user.id {
            cleanup_prepared(&prepared).await;
            return Err(AppError::Forbidden);
        }

        let selection = if file.odd_even == "custom" {
            match print_service::normalize_custom_page_range(
                upload.page_count,
                file.page_range.as_deref().unwrap_or_default(),
            ) {
                Ok(selection) => selection,
                Err(error) => {
                    cleanup_prepared(&prepared).await;
                    return Err(error);
                }
            }
        } else {
            file.odd_even
        };
        let page_count = match print_service::selected_page_count(upload.page_count, &selection) {
            Ok(count) => count,
            Err(error) => {
                cleanup_prepared(&prepared).await;
                return Err(error);
            }
        };
        let task_preview_path =
            utils::preview_dir(&state.config).join(format!("task-{}.pdf", Uuid::new_v4()));
        if let Err(error) = fs::copy(&upload.preview_path, &task_preview_path).await {
            cleanup_prepared(&prepared).await;
            return Err(error.into());
        }
        if let Err(error) =
            print_service::apply_page_selection(&task_preview_path, &selection).await
        {
            let _ = fs::remove_file(&task_preview_path).await;
            cleanup_prepared(&prepared).await;
            return Err(error);
        }
        prepared.push(PreparedFile {
            upload,
            odd_even: selection,
            page_count,
            task_preview_path,
        });
    }

    let submitted_pages = prepared
        .iter()
        .fold(0_i64, |total, file| total.saturating_add(file.page_count));
    let requires_review = print_service::submission_requires_review(
        used_today.saturating_add(reserved),
        submitted_pages,
        limit,
    );
    let client_ip = client_addr.ip().to_string();
    let persist_result: AppResult<Vec<SubmittedTask>> = async {
        let mut tx = state.pool.begin().await?;
        let mut tasks = Vec::with_capacity(prepared.len());
        for file in &prepared {
            let status = if requires_review {
                "pending_review"
            } else {
                "queued"
            };
            let result = sqlx::query(
            r#"
            INSERT INTO print_tasks
                (user_id, file_name, stored_path, preview_path, page_count, odd_even, status, submitted_ip, queued_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, CASE WHEN ? = 'queued' THEN datetime('now') END)
            "#,
        )
        .bind(user.id)
        .bind(&file.upload.original_name)
        .bind(&file.upload.stored_path)
        .bind(file.task_preview_path.to_string_lossy().to_string())
        .bind(file.page_count)
        .bind(&file.odd_even)
        .bind(status)
        .bind(&client_ip)
        .bind(status)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM temp_uploads WHERE id = ?")
            .bind(file.upload.id)
            .execute(&mut *tx)
            .await?;

        let id = result.last_insert_rowid();
        tasks.push(SubmittedTask {
            id,
            file_name: file.upload.original_name.clone(),
            page_count: file.page_count,
            odd_even: file.odd_even.clone(),
            status: status.to_string(),
            over_limit: requires_review,
        });
        }
        tx.commit().await?;
        Ok(tasks)
    }.await;

    let tasks = match persist_result {
        Ok(tasks) => tasks,
        Err(error) => {
            cleanup_prepared(&prepared).await;
            return Err(error);
        }
    };
    for file in &prepared {
        let _ = fs::remove_file(&file.upload.preview_path).await;
    }

    if let Err(error) = audit::log_with_ip(
        &state.pool,
        Some(user.id),
        "print.submit",
        &tasks,
        Some(&client_ip),
    )
    .await
    {
        warn!(
            ?error,
            user_id = user.id,
            "failed to write print submission audit log"
        );
    }
    state.broadcaster.send(QueueEvent::QueueChanged);

    Ok(Json(SubmitResponse {
        tasks,
        used_today,
        limit,
    }))
}

async fn cleanup_prepared(files: &[PreparedFile]) {
    for file in files {
        let _ = fs::remove_file(&file.task_preview_path).await;
    }
}

pub async fn cancel(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(task_id): Path<i64>,
) -> AppResult<Json<PrintTask>> {
    let _queue_guard = state.queue_lock.lock().await;
    let task = print_service::cancel_task(&state.pool, task_id, &user, None).await?;
    audit::log(&state.pool, Some(user.id), "print.cancel", &task).await?;
    state.broadcaster.send(QueueEvent::QueueChanged);
    Ok(Json(task))
}

fn default_odd_even() -> String {
    "all".to_string()
}

async fn serve_file(path: PathBuf, disposition: Option<&str>) -> AppResult<Response> {
    if !path.exists() {
        return Err(AppError::NotFound("文件已过期或不存在".into()));
    }
    let request = Request::new(Body::empty());
    let mut response = ServeFile::new(path)
        .oneshot(request)
        .await
        .map_err(|error| AppError::External(error.to_string()))?;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    if let Some(disposition) = disposition {
        response.headers_mut().insert(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(disposition)
                .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
        );
        if disposition.starts_with("inline") {
            allow_same_origin_preview(response.headers_mut());
        }
    }
    Ok(response.map(Body::new))
}

fn allow_same_origin_preview(headers: &mut HeaderMap) {
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("SAMEORIGIN"),
    );
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static("frame-ancestors 'self'"),
    );
}

fn upload_path(config: &crate::config::Config, original_name: &str) -> PathBuf {
    let extension = std::path::Path::new(original_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(utils::sanitize_filename)
        .filter(|ext| !ext.is_empty());
    let id = Uuid::new_v4();
    let name = extension
        .as_deref()
        .map(|extension| format!("{id}.{extension}"))
        .unwrap_or_else(|| id.to_string());
    utils::upload_dir(config).join(name)
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};

    use super::allow_same_origin_preview;

    #[test]
    fn preview_headers_allow_only_same_origin_framing() {
        let mut headers = HeaderMap::new();
        allow_same_origin_preview(&mut headers);

        assert_eq!(
            headers.get("x-frame-options"),
            Some(&HeaderValue::from_static("SAMEORIGIN"))
        );
        assert_eq!(
            headers.get("content-security-policy"),
            Some(&HeaderValue::from_static("frame-ancestors 'self'"))
        );
    }
}
