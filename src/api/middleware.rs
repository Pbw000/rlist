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

/// 管理员权限中间件 - 检查用户是否具有管理员权限
pub async fn admin_permission_middleware(
    state: State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    // 使用管理员权限（所有权限）进行检查
    let auth_config = state.inner.auth_config.clone();
    crate::auth::middleware::auth_permission_middleware_with_admin(
        state,
        request,
        next,
        auth_config,
    )
    .await
}
