//! 认证相关的 API 处理器

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::auth::auth::AuthConfig;

/// API 响应包装器
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 200,
            message: "success".to_string(),
            data: Some(data),
        }
    }

    pub fn error(code: i32, message: String) -> Self {
        Self {
            code,
            message,
            data: None,
        }
    }
}

impl<T: Serialize> axum::response::IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}

/// 注册请求
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 登录响应
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
}

/// 注册响应
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub message: String,
}

/// 用户注册
pub async fn register(
    State(auth_config): State<Arc<AuthConfig>>,
    Json(payload): Json<RegisterRequest>,
) -> impl axum::response::IntoResponse {
    match auth_config
        .register(payload.username, payload.password)
        .await
    {
        Ok(_user_id) => {
            let resp: ApiResponse<RegisterResponse> = ApiResponse::success(RegisterResponse {
                message: "注册成功".to_string(),
            });
            resp
        }
        Err((status, msg)) => {
            let resp: ApiResponse<RegisterResponse> =
                ApiResponse::error(status.as_u16() as i32, msg);
            resp
        }
    }
}

/// 用户登录
pub async fn login(
    State(auth_config): State<Arc<AuthConfig>>,
    Json(payload): Json<LoginRequest>,
) -> impl axum::response::IntoResponse {
    match auth_config.login(payload.username, payload.password).await {
        Ok(token) => {
            let resp: ApiResponse<LoginResponse> = ApiResponse::success(LoginResponse { token });
            resp
        }
        Err((status, msg)) => {
            let resp: ApiResponse<LoginResponse> = ApiResponse::error(status.as_u16() as i32, msg);
            resp
        }
    }
}

/// 获取当前用户信息
pub async fn get_current_user(
    _auth_config: State<Arc<AuthConfig>>,
) -> impl axum::response::IntoResponse {
    // 用户信息通过 middleware 注入到 request extensions
    // 这个端点可以用于验证 token 是否有效
    ApiResponse::success(serde_json::json!({
        "message": "已认证"
    }))
}
