use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub student_id: String,
    pub password_hash: String,
    pub role: String,
    pub qq: Option<String>,
    pub phone: Option<String>,
    pub status: String,
    pub must_change_password: bool,
    pub created_at: String,
    pub last_login_at: Option<String>,
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
    pub phone: Option<String>,
    pub status: String,
    pub must_change_password: bool,
    pub created_at: String,
    pub last_login_at: Option<String>,
}

impl From<User> for UserView {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            student_id: user.student_id,
            role: user.role,
            qq: user.qq,
            phone: user.phone,
            status: user.status,
            must_change_password: user.must_change_password,
            created_at: user.created_at,
            last_login_at: user.last_login_at,
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
            phone: user.phone.clone(),
            status: user.status.clone(),
            must_change_password: user.must_change_password,
            created_at: user.created_at.clone(),
            last_login_at: user.last_login_at.clone(),
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
    pub windows_job_id: Option<i64>,
    pub windows_job_name: Option<String>,
    pub printer_submitted_at: Option<String>,
    pub job_seen_at: Option<String>,
    pub status_detail: Option<String>,
}
