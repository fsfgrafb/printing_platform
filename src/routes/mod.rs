pub mod admin;
pub mod print;
pub mod queue;
pub mod user;

use axum::{
    extract::{DefaultBodyLimit, State},
    http::{header, HeaderName, HeaderValue},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use tower_http::{set_header::SetResponseHeaderLayer, trace::TraceLayer};

use crate::{
    app::AppState,
    auth,
    error::{AppError, AppResult},
    ws,
};

const INDEX_HTML: &str = include_str!("../../web/index.html");
const APP_JS: &str = include_str!("../../web/app.js");
const STYLES_CSS: &str = include_str!("../../web/styles.css");
const FAVICON: &[u8] = include_bytes!("../../web/favicon.svg");
const LOGO: &[u8] = include_bytes!("../../web/logo.svg");

pub fn router(state: AppState) -> Router {
    let body_limit = request_body_limit(&state.config);
    let authenticated = Router::new()
        .route("/auth/logout", post(auth::login::logout))
        .route("/auth/me", get(auth::login::me))
        .route("/auth/change-password", post(auth::login::change_password))
        .route("/ws/queue", get(ws::queue_ws))
        .merge(user::router())
        .merge(print::router())
        .merge(queue::router())
        .merge(admin::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::middleware::require_authenticated,
        ));
    let api = Router::new()
        .route("/auth/login", post(auth::login::login))
        .merge(health_router())
        .merge(authenticated)
        .fallback(api_not_found)
        .layer(DefaultBodyLimit::max(body_limit))
        .layer(TraceLayer::new_for_http());

    Router::new()
        .nest("/api", api)
        .route("/", get(index))
        .route("/app.js", get(app_js))
        .route("/styles.css", get(styles_css))
        .route("/favicon.svg", get(favicon))
        .route("/logo.svg", get(logo))
        .fallback(index)
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("same-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self' ws: wss:; img-src 'self' data:; font-src 'self'; frame-src 'self'; object-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
            ),
        ))
        .with_state(state)
}

fn request_body_limit(config: &crate::config::Config) -> usize {
    let file_count = u64::try_from(config.limits.max_files_per_request).unwrap_or(u64::MAX);
    let multi_upload_bytes = config
        .limits
        .max_upload_bytes
        .saturating_mul(file_count)
        .min(config.limits.max_temp_bytes_per_user);
    usize::try_from(
        multi_upload_bytes
            .max(config.limits.max_import_bytes)
            .saturating_add(1024 * 1024),
    )
    .unwrap_or(usize::MAX)
}

async fn index() -> Response {
    static_asset("text/html; charset=utf-8", INDEX_HTML.as_bytes())
}

async fn app_js() -> Response {
    static_asset("text/javascript; charset=utf-8", APP_JS.as_bytes())
}

async fn styles_css() -> Response {
    static_asset("text/css; charset=utf-8", STYLES_CSS.as_bytes())
}

async fn favicon() -> Response {
    static_asset("image/svg+xml", FAVICON)
}

async fn logo() -> Response {
    static_asset("image/svg+xml", LOGO)
}

fn static_asset(content_type: &'static str, body: &'static [u8]) -> Response {
    (
        [
            (header::CONTENT_TYPE, HeaderValue::from_static(content_type)),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ),
        ],
        body,
    )
        .into_response()
}

async fn api_not_found() -> AppError {
    AppError::NotFound("api endpoint not found".to_string())
}

fn health_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(live))
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
}

#[derive(Debug, Serialize)]
struct LiveResponse {
    ok: bool,
    version: &'static str,
}

#[derive(Debug, Serialize)]
struct ReadyResponse {
    ok: bool,
    database: &'static str,
}

async fn live() -> Json<LiveResponse> {
    Json(LiveResponse {
        ok: true,
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn ready(State(state): State<AppState>) -> AppResult<Json<ReadyResponse>> {
    sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(ReadyResponse {
        ok: true,
        database: "ok",
    }))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body},
        http::{header, Request, StatusCode},
    };
    use sqlx::sqlite::SqlitePoolOptions;
    use tower::ServiceExt;

    use super::{request_body_limit, router};
    use crate::{app::AppState, config::Config};

    async fn test_app() -> axum::Router {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        router(AppState::new(pool, Config::default()))
    }

    #[test]
    fn request_body_limit_allows_multiple_upload_files() {
        let config = Config::default();
        assert_eq!(
            request_body_limit(&config),
            201 * 1024 * 1024,
            "the default request limit should allow four 50 MiB files plus multipart overhead"
        );
    }

    #[tokio::test]
    async fn embedded_frontend_is_served_without_external_files() {
        let response = test_app()
            .await
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
        let csp = response
            .headers()
            .get("content-security-policy")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(csp.contains("script-src 'self'"));
        assert!(csp.contains("style-src 'self'"));
        assert!(csp.contains("frame-src 'self'"));
        assert!(!csp.contains("'unsafe-inline'"));
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("/app.js"));
        assert!(html.contains("/favicon.svg"));
        assert!(html.contains("<title>ACM 实验室自助打印平台</title>"));
    }

    #[tokio::test]
    async fn embedded_frontend_respects_the_csp_without_stale_asset_caching() {
        let response = test_app()
            .await
            .oneshot(Request::get("/app.js").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
        let body = to_bytes(response.into_body(), 2 * 1024 * 1024)
            .await
            .unwrap();
        let javascript = String::from_utf8(body.to_vec()).unwrap();
        assert!(!javascript.contains("style="));
        assert!(!javascript.contains("javascript:"));
        assert!(javascript.contains("<svg class=\"quota-track\""));
        assert!(javascript.contains("await animateRouteLeave(view)"));
        assert!(javascript.contains("await renderRoute({ animate: true })"));
        assert!(javascript.contains("modal.className = 'preview-modal'"));
        assert!(!javascript.contains("window.open(previewUrl"));
        assert!(javascript.contains("addEventListener('click', () => closePreview())"));
        assert!(javascript.contains("data-range=\"custom\""));
        assert!(javascript.contains("parseCustomPageRange"));
        assert!(javascript.contains("function openActionDialog"));
        assert!(javascript.contains("<th>角色</th><th class=\"user-centered\">状态</th>"));
        assert!(javascript.contains("id=\"user-filter\" class=\"button-row user-toolbar\""));
        assert!(!javascript.contains("<select name=\"role\">"));
        assert!(javascript.contains("placeholder=\"按学号筛选\""));
        assert!(javascript.contains("app.dataset.shellKey === shellKey"));
        assert!(javascript.contains("function printerStatusDisplay"));
        assert!(javascript.contains("modal.classList.add('closing')"));
        assert!(javascript.contains("function updateNavHighlight"));
        assert!(javascript.contains("uploads.push(...pendingUploads)"));
        assert!(javascript.contains("appendUploadCards(pendingUploads)"));
        assert!(javascript.contains("class=\"upload-spinner\""));
        assert!(javascript.contains("pendingUploads.map(async (pendingUpload) =>"));
        assert!(javascript.contains("data.append('files', pendingUpload.source_file)"));
        assert!(javascript.contains("status: 'ready'"));
        assert!(javascript.contains("replaceUploadCard(pendingUpload)"));
        assert!(javascript.contains("'提交审批'"));
        assert!(javascript.contains("class=\"review-required\">需审批"));
        assert!(javascript.contains("uploadList.scrollTo({ top: uploadList.scrollHeight"));
        assert!(javascript.contains("无法连接服务器，请检查网络连接或确认程序正在运行"));
    }

    #[tokio::test]
    async fn business_routes_are_protected_as_a_group() {
        let response = test_app()
            .await
            .oneshot(
                Request::get("/api/user/admin-contact")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
