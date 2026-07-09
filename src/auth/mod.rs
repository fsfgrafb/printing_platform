pub mod login;
pub mod middleware;
pub mod session;

use crate::{
    db::models::User,
    error::{AppError, AppResult},
};

pub fn ensure_admin(user: &User) -> AppResult<()> {
    if user.is_admin() {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
