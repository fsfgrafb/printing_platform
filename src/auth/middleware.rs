use axum::{
    async_trait,
    body::Body,
    extract::{FromRequestParts, Request, State},
    http::request::Parts,
    middleware::Next,
    response::Response,
};
use axum_extra::extract::cookie::CookieJar;

use crate::{
    app::AppState,
    auth::session::{self, COOKIE_NAME},
    db::models::User,
    error::AppError,
};

pub struct CurrentUser(pub User);

pub async fn require_authenticated(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let jar = CookieJar::from_headers(request.headers());
    let Some(cookie) = jar.get(COOKIE_NAME) else {
        return Err(AppError::Unauthorized);
    };
    let Some(user) =
        session::authenticate(&state.pool, cookie.value(), state.config.session_days).await?
    else {
        return Err(AppError::Unauthorized);
    };
    let path = request.uri().path();
    if user.must_change_password
        && !path.ends_with("/auth/me")
        && !path.ends_with("/auth/change-password")
        && !path.ends_with("/auth/logout")
    {
        return Err(AppError::Conflict("首次登录必须先修改密码".to_string()));
    }
    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

#[async_trait]
impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<User>()
            .cloned()
            .map(CurrentUser)
            .ok_or(AppError::Unauthorized)
    }
}
