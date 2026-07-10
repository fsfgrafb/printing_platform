use std::{collections::HashSet, net::SocketAddr, path::PathBuf};

use axum::{
    body::Body,
    extract::{ConnectInfo, Multipart, Path, State},
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncWriteExt};
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
        .route("/print/submit", post(submit))
        .route("/print/tasks/:task_id", delete(cancel))
}

pub async fn list_uploads(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<UploadResponse>> {
    let uploads = sqlx::query_as::<_, TempUpload>(
        r#"
        SELECT id, temp_id, user_id, original_name, stored_path, preview_path, page_count, created_at
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

    while let Some(mut field) = multipart.next_field().await? {
        if field.file_name().is_none() {
            continue;
        }

        let original_name = field.file_name().unwrap_or("upload.bin").to_string();
        let safe_name = utils::sanitize_filename(&original_name);
        let temp_id = Uuid::new_v4().to_string();
        let stored_path = utils::upload_dir(&state.config).join(format!("{temp_id}_{safe_name}"));
        let preview_path = utils::preview_dir(&state.config).join(format!("{temp_id}.pdf"));

        info!(%temp_id, file_name = %original_name, "receiving uploaded file");
        let mut output = fs::File::create(&stored_path).await?;
        while let Some(chunk) = field.chunk().await? {
            output.write_all(&chunk).await?;
        }
        output.flush().await?;
        drop(output);
        info!(%temp_id, path = %stored_path.display(), "uploaded file saved");

        let page_count =
            match converter::prepare_preview(&state.config, &stored_path, &preview_path).await {
                Ok(count) => count,
                Err(error) => {
                    let _ = fs::remove_file(&stored_path).await;
                    let _ = fs::remove_file(&preview_path).await;
                    return Err(error);
                }
            };
        info!(%temp_id, page_count, preview = %preview_path.display(), "preview ready");
        sqlx::query(
            r#"
            INSERT INTO temp_uploads (temp_id, user_id, original_name, stored_path, preview_path, page_count)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&temp_id)
        .bind(user.id)
        .bind(&original_name)
        .bind(stored_path.to_string_lossy().to_string())
        .bind(preview_path.to_string_lossy().to_string())
        .bind(page_count)
        .execute(&state.pool)
        .await?;

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
        SELECT id, temp_id, user_id, original_name, stored_path, preview_path, page_count, created_at
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

    let bytes = fs::read(path).await?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/pdf"),
    );
    Ok((headers, Body::from(bytes)).into_response())
}

pub async fn task_preview(
    State(state): State<AppState>,
    CurrentUser(_user): CurrentUser,
    Path(task_id): Path<i64>,
) -> AppResult<Response> {
    let preview_path: Option<String> =
        sqlx::query_scalar("SELECT preview_path FROM print_tasks WHERE id = ?")
            .bind(task_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("print record not found".to_string()))?;

    let path = preview_path
        .map(PathBuf::from)
        .ok_or_else(|| AppError::NotFound("final PDF is not available".to_string()))?;
    let bytes = fs::read(&path).await.map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            AppError::NotFound("final PDF has expired or is missing".to_string())
        } else {
            error.into()
        }
    })?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/pdf"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("inline; filename=print-preview.pdf"),
    );
    Ok((headers, Body::from(bytes)).into_response())
}

pub async fn remove_upload(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(temp_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let upload = sqlx::query_as::<_, TempUpload>(
        r#"
        SELECT id, temp_id, user_id, original_name, stored_path, preview_path, page_count, created_at
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
    headers: HeaderMap,
    Json(request): Json<SubmitRequest>,
) -> AppResult<Json<SubmitResponse>> {
    if request.files.is_empty() {
        return Err(AppError::BadRequest("files cannot be empty".to_string()));
    }

    let mut temp_ids = HashSet::new();
    for file in &request.files {
        if !matches!(file.odd_even.as_str(), "all" | "odd" | "even") {
            return Err(AppError::BadRequest(
                "odd_even must be one of all, odd, even".to_string(),
            ));
        }
        if !temp_ids.insert(file.temp_id.as_str()) {
            return Err(AppError::BadRequest(
                "duplicate temp_id in submit request".to_string(),
            ));
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
            SELECT id, temp_id, user_id, original_name, stored_path, preview_path, page_count, created_at
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

        let page_count = match print_service::selected_page_count(upload.page_count, &file.odd_even)
        {
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
            print_service::apply_page_selection(&task_preview_path, &file.odd_even).await
        {
            let _ = fs::remove_file(&task_preview_path).await;
            cleanup_prepared(&prepared).await;
            return Err(error);
        }
        prepared.push(PreparedFile {
            upload,
            odd_even: file.odd_even,
            page_count,
            task_preview_path,
        });
    }

    let client_ip = request_ip(&headers, client_addr);
    let persist_result: AppResult<Vec<SubmittedTask>> = async {
        let mut projected_used = used_today + reserved;
        let mut tx = state.pool.begin().await?;
        let mut tasks = Vec::with_capacity(prepared.len());
        for file in &prepared {
            let (status, over_limit) = print_service::quota_status(&mut projected_used, file.page_count, limit);
            let result = sqlx::query(
            r#"
            INSERT INTO print_tasks
                (user_id, file_name, stored_path, preview_path, page_count, odd_even, status, submitted_ip)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
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
            over_limit,
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

fn request_ip(headers: &HeaderMap, peer: SocketAddr) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| peer.ip().to_string())
}
