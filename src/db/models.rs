use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub student_id: String,
    pub password_hash: String,
    pub role: String,
    pub qq: Option<String>,
    pub must_change_password: bool,
    pub created_at: String,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

#[derive(Debug, Serialize)]
pub struct UserView {
    pub id: i64,
    pub student_id: String,
    pub role: String,
    pub qq: Option<String>,
    pub must_change_password: bool,
    pub created_at: String,
}

impl From<User> for UserView {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            student_id: user.student_id,
            role: user.role,
            qq: user.qq,
            must_change_password: user.must_change_password,
            created_at: user.created_at,
        }
    }
}

impl From<&User> for UserView {
    fn from(user: &User) -> Self {
        Self {
            id: user.id,
            student_id: user.student_id.clone(),
            role: user.role.clone(),
            qq: user.qq.clone(),
            must_change_password: user.must_change_password,
            created_at: user.created_at.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TempUpload {
    pub id: i64,
    pub temp_id: String,
    pub user_id: i64,
    pub original_name: String,
    pub stored_path: String,
    pub preview_path: String,
    pub page_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PrintTask {
    pub id: i64,
    pub user_id: i64,
    pub file_name: String,
    pub stored_path: String,
    pub preview_path: Option<String>,
    pub page_count: i64,
    pub odd_even: String,
    pub status: String,
    pub submitted_at: String,
    pub completed_at: Option<String>,
    pub cancelled_by: Option<String>,
    pub review_reason: Option<String>,
    pub approved_over_quota: bool,
}
