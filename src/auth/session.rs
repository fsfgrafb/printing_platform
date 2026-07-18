use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use rand_core::OsRng;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    db::models::User,
    error::{AppError, AppResult},
};

pub const COOKIE_NAME: &str = "session_token";

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| AppError::Password(error.to_string()))
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

pub async fn create_session(
    pool: &SqlitePool,
    user_id: i64,
    session_days: i64,
) -> AppResult<String> {
    let token = Uuid::new_v4().to_string();
    let expires_at = (Utc::now() + Duration::days(session_days)).to_rfc3339();

    sqlx::query("INSERT INTO sessions (token, user_id, expires_at) VALUES (?, ?, ?)")
        .bind(&token)
        .bind(user_id)
        .bind(expires_at)
        .execute(pool)
        .await?;

    Ok(token)
}

pub async fn delete_session(pool: &SqlitePool, token: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM sessions WHERE token = ?")
        .bind(token)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn authenticate(
    pool: &SqlitePool,
    token: &str,
    session_days: i64,
) -> AppResult<Option<User>> {
    let now = Utc::now().to_rfc3339();
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT u.id, u.student_id, u.password_hash, u.role, u.qq, u.phone, u.status,
               u.must_change_password, u.created_at, u.last_login_at
        FROM sessions s
        JOIN users u ON u.id = s.user_id
        WHERE s.token = ? AND s.expires_at > ? AND u.status != 'banned'
        "#,
    )
    .bind(token)
    .bind(&now)
    .fetch_optional(pool)
    .await?;

    if user.is_some() {
        let expires_at = (Utc::now() + Duration::days(session_days)).to_rfc3339();
        sqlx::query("UPDATE sessions SET expires_at = ? WHERE token = ?")
            .bind(expires_at)
            .bind(token)
            .execute(pool)
            .await?;
    }

    Ok(user)
}

pub async fn delete_user_sessions(pool: &SqlitePool, user_id: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM sessions WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
