use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("authentication required")]
    Unauthorized,
    #[error("permission denied")]
    Forbidden,
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error(transparent)]
    Multipart(#[from] axum::extract::multipart::MultipartError),
    #[error("{0}")]
    Password(String),
    #[error("{0}")]
    External(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let message = self.to_string();
        let status = match self {
            AppError::BadRequest(_) | AppError::Multipart(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Io(_)
            | AppError::Sqlx(_)
            | AppError::Toml(_)
            | AppError::Password(_)
            | AppError::External(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        if status.is_server_error() {
            tracing::error!(%status, error = %message, "request failed");
        } else if status != StatusCode::UNAUTHORIZED {
            tracing::warn!(%status, error = %message, "request rejected");
        }

        let public_message = if status.is_server_error() {
            "服务器内部错误，请联系管理员".to_string()
        } else {
            message
        };
        let body = Json(ErrorBody {
            error: public_message,
        });
        (status, body).into_response()
    }
}
