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

        Ok(CurrentUser(user))
    }
}
