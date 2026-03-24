//! 认证相关的 API 处理器

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::state::AppState;
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
    pub salt: u64,
    pub timestamp: u64,
    pub claim: String,
}

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub salt: u64,
    pub timestamp: u64,
    pub claim: String,
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
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> impl axum::response::IntoResponse {
    let challenge = &state.inner.challenge;

    // 验证时间戳
    if let Err(err) = challenge.validate_timestamp(payload.timestamp) {
        let resp: ApiResponse<RegisterResponse> =
            ApiResponse::error(400, format!("时间戳无效：{}", err));
        return resp;
    }

    // 验证 challenge (payload = timestamp + username + password)
    let challenge_payload = format!(
        "{}{}{}",
        payload.timestamp, payload.username, payload.password
    );
    if let Err(_) = challenge
        .validate(payload.salt, &payload.claim, &challenge_payload)
        .await
    {
        let resp: ApiResponse<RegisterResponse> =
            ApiResponse::error(400, "Challenge 验证失败".to_string());
        return resp;
    }

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
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> impl axum::response::IntoResponse {
    let challenge = &state.inner.challenge;

    // 验证时间戳
    if let Err(err) = challenge.validate_timestamp(payload.timestamp) {
        let resp: ApiResponse<LoginResponse> =
            ApiResponse::error(400, format!("时间戳无效：{}", err));
        return resp;
    }

    // 验证 challenge (payload = timestamp + username + password)
    let challenge_payload = format!(
        "{}{}{}",
        payload.timestamp, payload.username, payload.password
    );
    if let Err(_) = challenge
        .validate(payload.salt, &payload.claim, &challenge_payload)
        .await
    {
        let resp: ApiResponse<LoginResponse> =
            ApiResponse::error(400, "Challenge 验证失败".to_string());
        return resp;
    }

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

/// Challenge 响应
#[derive(Debug, Serialize)]
pub struct ChallengeResponse {
    pub salt: u64,
}

/// 获取 Challenge
pub async fn get_challenge(State(state): State<AppState>) -> impl axum::response::IntoResponse {
    let salt_value = state.inner.challenge.challenge.get_current_salt();
    ApiResponse::success(ChallengeResponse { salt: salt_value })
}
