use axum::{extract::State, Json};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use time::Duration as CookieDuration;

use crate::{
    app::AppState,
    auth::{
        middleware::CurrentUser,
        session::{self, COOKIE_NAME},
    },
    config::Config,
    db::models::{User, UserView},
    error::{AppError, AppResult},
};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub student_id: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: UserView,
    pub min_password_length: usize,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub new_password: String,
    pub confirm_password: String,
}

pub async fn ensure_initial_admin(pool: &SqlitePool, config: &Config) -> AppResult<()> {
    let admin_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin'")
        .fetch_one(pool)
        .await?;

    if admin_count > 0 {
        return Ok(());
    }

    let student_id = config.initial_admin_student_id.trim();
    if student_id.is_empty() {
        return Err(AppError::BadRequest(
            "initial_admin_student_id cannot be empty".to_string(),
        ));
    }

    let hash = session::hash_password(student_id)?;
    sqlx::query(
        r#"
        INSERT INTO users (student_id, password_hash, role, must_change_password)
        VALUES (?, ?, 'admin', 1)
        ON CONFLICT(student_id) DO UPDATE SET role = 'admin'
        "#,
    )
    .bind(student_id)
    .bind(hash)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(request): Json<LoginRequest>,
) -> AppResult<(CookieJar, Json<LoginResponse>)> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, student_id, password_hash, role, qq, phone, status,
               must_change_password, created_at, last_login_at
        FROM users
        WHERE student_id = ?
        "#,
    )
    .bind(request.student_id.trim())
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    if !session::verify_password(&user.password_hash, &request.password) {
        return Err(AppError::Unauthorized);
    }

    if user.status == "banned" {
        return Err(AppError::Conflict("账号已被封禁，请联系管理员".to_string()));
    }

    sqlx::query(
        "UPDATE users SET last_login_at = datetime('now'), status = CASE WHEN status = 'unused' THEN 'normal' ELSE status END WHERE id = ?",
    )
        .bind(user.id)
        .execute(&state.pool)
        .await?;

    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, student_id, password_hash, role, qq, phone, status,
               must_change_password, created_at, last_login_at
        FROM users
        WHERE id = ?
        "#,
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;

    let token = session::create_session(&state.pool, user.id, state.config.session_days).await?;
    let jar = jar.add(session_cookie(&token, state.config.session_days));

    Ok((
        jar,
        Json(LoginResponse {
            user: user.into(),
            min_password_length: state.config.min_password_length,
        }),
    ))
}

pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> AppResult<CookieJar> {
    if let Some(cookie) = jar.get(COOKIE_NAME) {
        session::delete_session(&state.pool, cookie.value()).await?;
    }

    Ok(jar.remove(remove_cookie()))
}

pub async fn me(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<LoginResponse>> {
    Ok(Json(LoginResponse {
        user: user.into(),
        min_password_length: state.config.min_password_length,
    }))
}

pub async fn change_password(
    State(state): State<AppState>,
    jar: CookieJar,
    CurrentUser(user): CurrentUser,
    Json(request): Json<ChangePasswordRequest>,
) -> AppResult<(CookieJar, Json<serde_json::Value>)> {
    if request.new_password.chars().count() < state.config.min_password_length {
        return Err(AppError::BadRequest(format!(
            "新密码至少需要 {} 个字符",
            state.config.min_password_length
        )));
    }

    if request.new_password != request.confirm_password {
        return Err(AppError::BadRequest("两次输入的密码不一致".to_string()));
    }

    let hash = session::hash_password(&request.new_password)?;
    sqlx::query("UPDATE users SET password_hash = ?, must_change_password = 0 WHERE id = ?")
        .bind(hash)
        .bind(user.id)
        .execute(&state.pool)
        .await?;
    session::delete_user_sessions(&state.pool, user.id).await?;

    Ok((
        jar.remove(remove_cookie()),
        Json(serde_json::json!({ "ok": true, "relogin_required": true })),
    ))
}

fn session_cookie(token: &str, session_days: i64) -> Cookie<'static> {
    let mut cookie = Cookie::new(COOKIE_NAME, token.to_string());
    cookie.set_http_only(true);
    cookie.set_path("/");
    cookie.set_same_site(SameSite::Lax);
    cookie.set_max_age(CookieDuration::days(session_days));
    cookie
}

fn remove_cookie() -> Cookie<'static> {
    let mut cookie = Cookie::new(COOKIE_NAME, "");
    cookie.set_path("/");
    cookie.set_max_age(CookieDuration::seconds(0));
    cookie
}
