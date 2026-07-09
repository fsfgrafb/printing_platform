use std::{collections::HashSet, path::PathBuf};

use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncWriteExt};
use uuid::Uuid;

use crate::{
    app::AppState,
    auth::middleware::CurrentUser,
    db::models::{PrintTask, TempUpload},
    error::{AppError, AppResult},
    services::{audit, converter, print_service, quota},
    utils::file,
    ws::QueueEvent,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/print/upload", post(upload))
        .route("/print/preview/:temp_id", get(preview))
        .route("/print/submit", post(submit))
        .route("/print/tasks/:task_id", delete(cancel))
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
        let safe_name = file::sanitize_filename(&original_name);
        let temp_id = Uuid::new_v4().to_string();
        let stored_path = file::upload_dir(&state.config).join(format!("{temp_id}_{safe_name}"));
        let preview_path = file::preview_dir(&state.config).join(format!("{temp_id}.pdf"));

        let mut output = fs::File::create(&stored_path).await?;
        while let Some(chunk) = field.chunk().await? {
            output.write_all(&chunk).await?;
        }
        output.flush().await?;

        let page_count =
            converter::prepare_preview(&state.config, &stored_path, &preview_path).await?;
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

pub async fn submit(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
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

    let used_today = quota::used_today(&state.pool, user.id).await?;
    let limit = quota::daily_limit(&state.pool).await?;
    let mut projected_used = used_today;
    let mut tx = state.pool.begin().await?;
    let mut tasks = Vec::new();

    for file in request.files {
        let upload = sqlx::query_as::<_, TempUpload>(
            r#"
            SELECT id, temp_id, user_id, original_name, stored_path, preview_path, page_count, created_at
            FROM temp_uploads
            WHERE temp_id = ?
            "#,
        )
        .bind(&file.temp_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("uploaded file not found".to_string()))?;

        if upload.user_id != user.id {
            return Err(AppError::Forbidden);
        }

        let page_count = print_service::selected_page_count(upload.page_count, &file.odd_even)?;
        let preview_path = PathBuf::from(&upload.preview_path);
        print_service::apply_page_selection(&preview_path, &file.odd_even).await?;
        let (status, over_limit) =
            print_service::quota_status(&mut projected_used, page_count, limit);

        let result = sqlx::query(
            r#"
            INSERT INTO print_tasks
                (user_id, file_name, stored_path, preview_path, page_count, odd_even, status)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(user.id)
        .bind(&upload.original_name)
        .bind(&upload.stored_path)
        .bind(&upload.preview_path)
        .bind(page_count)
        .bind(&file.odd_even)
        .bind(status)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM temp_uploads WHERE id = ?")
            .bind(upload.id)
            .execute(&mut *tx)
            .await?;

        let id = result.last_insert_rowid();
        tasks.push(SubmittedTask {
            id,
            file_name: upload.original_name,
            page_count,
            odd_even: file.odd_even,
            status: status.to_string(),
            over_limit,
        });
    }

    tx.commit().await?;

    audit::log(&state.pool, Some(user.id), "print.submit", &tasks).await?;
    state.broadcaster.send(QueueEvent::QueueChanged);

    Ok(Json(SubmitResponse {
        tasks,
        used_today,
        limit,
    }))
}

pub async fn cancel(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(task_id): Path<i64>,
) -> AppResult<Json<PrintTask>> {
    let task = print_service::cancel_task(&state.pool, task_id, &user, None).await?;
    audit::log(&state.pool, Some(user.id), "print.cancel", &task).await?;
    state.broadcaster.send(QueueEvent::QueueChanged);
    Ok(Json(task))
}

fn default_odd_even() -> String {
    "all".to_string()
}
