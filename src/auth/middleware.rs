use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use axum_extra::extract::cookie::CookieJar;

use crate::{
    app::AppState,
    auth::session::{self, COOKIE_NAME},
    db::models::User,
    error::AppError,
};

pub struct CurrentUser(pub User);

#[async_trait]
impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let Some(cookie) = jar.get(COOKIE_NAME) else {
            return Err(AppError::Unauthorized);
        };

        let Some(user) =
            session::authenticate(&state.pool, cookie.value(), state.config.session_days).await?
        else {
            return Err(AppError::Unauthorized);
        };

        // A default password must not grant access to any business endpoint.
        // `/auth/me` is needed to restore the client session and the password
        // endpoint itself must remain reachable.
        let path = parts.uri.path();
        if user.must_change_password
            && !path.ends_with("/auth/me")
            && !path.ends_with("/auth/change-password")
        {
            return Err(AppError::Conflict("首次登录必须先修改密码".to_string()));
        }

        Ok(CurrentUser(user))
    }
}
