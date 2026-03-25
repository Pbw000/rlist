//! JWT 认证和权限检查中间件

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::warn;

use crate::api::state::AppState;
use crate::auth::auth::{AuthClaim, AuthConfig, Permission};
use crate::auth::jwt::verify_token;
use crate::auth::user_store::UserPermissions;

/// 认证和权限检查中间件状态
#[derive(Clone)]
pub struct AuthMiddlewareState {
    pub auth_config: Arc<AuthConfig>,
    pub required_permission: Permission,
}

pub async fn auth_permission_middleware(
    State(state): State<AuthMiddlewareState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let (parts, body) = request.into_parts();
    let token = headers
        .get("AUTH-JWT-TOKEN")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            let client_ip = headers
                .get("cf-connecting-ip")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");
            let method = parts.method.as_str();
            let request_path = parts.uri.path();
            warn!(
                target: "security",
                client_ip = %client_ip,
                method = %method,
                request_path = %request_path,
                "Authentication failed: Missing AUTH-JWT-TOKEN header"
            );
            StatusCode::UNAUTHORIZED
        })?;

    let claim = match verify_token::<AuthClaim>(token, &state.auth_config.secret_key) {
        Ok(claim) => claim,
        Err(err) => {
            let client_ip = headers
                .get("cf-connecting-ip")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");
            let method = parts.method.as_str();
            let request_path = parts.uri.path();
            warn!(
                target: "security",
                client_ip = %client_ip,
                method = %method,
                request_path = %request_path,
                error = %err,
                "Authentication failed: Jwt Verify failed"
            );
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // 获取用户信息
    let user_info = state
        .auth_config
        .users
        .read()
        .await
        .get(&claim.i)
        .cloned()
        .ok_or_else(|| {
            let client_ip = headers
                .get("cf-connecting-ip")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");
            let method = parts.method.as_str();
            let request_path = parts.uri.path();
            warn!(
                target: "security",
                client_ip = %client_ip,
                method = %method,
                request_path = %request_path,
                "Authentication failed: Invalid user ID in JWT claim"
            );
            StatusCode::UNAUTHORIZED
        })?;

    // 检查权限
    if !user_info.has_permission(&state.required_permission) {
        warn!(
            target: "security",
            client_ip = %parts.headers.get("cf-connecting-ip").and_then(|v| v.to_str().ok()).unwrap_or("unknown"),
            method = %parts.method.as_str(),
            path = %parts.uri.path(),
            user = %user_info.user_name,
            "Permission denied: User lacks {:?} permission",
            state.required_permission
        );
        return Err(StatusCode::FORBIDDEN);
    }

    let mut request = Request::from_parts(parts, body);
    request.extensions_mut().insert(user_info);
    Ok(next.run(request).await)
}

pub async fn auth_permission_middleware_with_admin(
    _state: State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
    auth_config: Arc<AuthConfig>,
) -> Result<Response, StatusCode> {
    let (parts, body) = request.into_parts();
    let headers = &parts.headers;

    let token = headers
        .get("AUTH-JWT-TOKEN")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            let client_ip = headers
                .get("cf-connecting-ip")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");
            let method = parts.method.as_str();
            let request_path = parts.uri.path();
            warn!(
                target: "security",
                client_ip = %client_ip,
                method = %method,
                request_path = %request_path,
                "Authentication failed: Missing AUTH-JWT-TOKEN header"
            );
            StatusCode::UNAUTHORIZED
        })?;

    let claim = match verify_token::<AuthClaim>(token, &auth_config.secret_key) {
        Ok(claim) => claim,
        Err(err) => {
            let client_ip = headers
                .get("cf-connecting-ip")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");
            let method = parts.method.as_str();
            let request_path = parts.uri.path();
            warn!(
                target: "security",
                client_ip = %client_ip,
                method = %method,
                request_path = %request_path,
                error = %err,
                "Authentication failed: Jwt Verify failed"
            );
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // 获取用户信息
    let user_info = auth_config
        .users
        .read()
        .await
        .get(&claim.i)
        .cloned()
        .ok_or_else(|| {
            let client_ip = headers
                .get("cf-connecting-ip")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");
            let method = parts.method.as_str();
            let request_path = parts.uri.path();
            warn!(
                target: "security",
                client_ip = %client_ip,
                method = %method,
                request_path = %request_path,
                "Authentication failed: Invalid user ID in JWT claim"
            );
            StatusCode::UNAUTHORIZED
        })?;
    if user_info.user_name != "admin" {
        warn!(
            target: "security",
            client_ip = %parts.headers.get("cf-connecting-ip").and_then(|v| v.to_str().ok()).unwrap_or("unknown"),
            method = %parts.method.as_str(),
            path = %parts.uri.path(),
            user = %user_info.user_name,
            "Permission denied: User name is not admin"
        );
        return Err(StatusCode::FORBIDDEN);
    }
    let user_permissions = auth_config
        .credentials_store
        .get_permissions(&user_info.user_name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let admin_permissions = UserPermissions::admin();
    if user_permissions.to_bits() != admin_permissions.to_bits() {
        warn!(
            target: "security",
            client_ip = %parts.headers.get("cf-connecting-ip").and_then(|v| v.to_str().ok()).unwrap_or("unknown"),
            method = %parts.method.as_str(),
            path = %parts.uri.path(),
            user = %user_info.user_name,
            "Permission denied: User is not admin"
        );
        return Err(StatusCode::FORBIDDEN);
    }

    let mut request = Request::from_parts(parts, body);
    request.extensions_mut().insert(user_info);
    Ok(next.run(request).await)
}
