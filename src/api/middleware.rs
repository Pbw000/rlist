//! 认证中间件
//!
//! 提供特定路由的访问控制

use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::api::state::AppState;

pub async fn admin_auth_middleware(
    state: State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    let admin_key = request
        .headers()
        .get("X-Admin-Key")
        .and_then(|h| h.to_str().ok());

    if let Some(key) = admin_key {
        if state.verify_admin_key(key) {
            return Ok(next.run(request).await);
        }
    }
    Err((StatusCode::UNAUTHORIZED, "需要管理员权限"))
}
