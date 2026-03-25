//! 管理员处理器 - 需要管理员权限

use crate::{
    api::{state::AppState, types::*},
    auth::challenge::IntoHashContext,
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use ring::digest::SHA512;

use crate::api::types::ApiResponse;

/// 添加存储
pub async fn add_storage(
    State(_state): State<AppState>,
    Json(_req): Json<StorageConfigRequest>,
) -> impl IntoResponse {
    ApiResponse::success(serde_json::json!({"message": "存储添加功能开发中"}))
}

pub async fn remove_storage(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    state.remove_storage(&name).await;
    ApiResponse::success(serde_json::json!({"deleted": name}))
}

/// 列出所有用户
pub async fn list_users(State(state): State<AppState>) -> impl IntoResponse {
    match state
        .inner
        .auth_config
        .credentials_store
        .list_usernames()
        .await
    {
        Ok(usernames) => {
            let mut users = Vec::new();
            for username in usernames {
                match state
                    .inner
                    .auth_config
                    .credentials_store
                    .get_permissions(&username)
                    .await
                {
                    Ok(perms) => {
                        users.push(UserInfoResponse {
                            username,
                            permissions: perms.into(),
                        });
                    }
                    Err(_) => {
                        // 跳过无法获取权限的用户
                        continue;
                    }
                }
            }
            ApiResponse::success(users)
        }
        Err(e) => ApiResponse::error(500, format!("获取用户列表失败：{}", e)),
    }
}

pub async fn remove_user(
    State(state): State<AppState>,
    Json(req): Json<RmUserRequest>,
) -> impl IntoResponse {
    // 不允许删除 admin 用户
    if req.user_name == "admin" {
        return ApiResponse::error(400, "不能删除 admin 用户".to_string());
    }

    let success = state.inner.auth_config.remove_user(&req.user_name).await;
    if success {
        ApiResponse::success(serde_json::json!({"deleted": req.user_name}))
    } else {
        ApiResponse::error(404, "User not found".to_string())
    }
}

/// 修改用户权限
pub async fn update_user_permissions(
    State(state): State<AppState>,
    Json(req): Json<UpdatePermissionsRequest>,
) -> impl IntoResponse {
    if req.user_name == "admin" {
        return ApiResponse::error(400, "不能修改 admin 用户的权限".to_string());
    }

    // 检查用户是否存在
    if !state
        .inner
        .auth_config
        .credentials_store
        .exists(&req.user_name)
        .await
    {
        return ApiResponse::error(404, "用户不存在".to_string());
    }

    match state
        .inner
        .auth_config
        .credentials_store
        .update_permissions(&req.user_name, req.permissions.into())
        .await
    {
        Ok(_) => ApiResponse::success(serde_json::json!({
            "message": "权限更新成功",
            "username": req.user_name
        })),
        Err(e) => ApiResponse::error(500, format!("更新权限失败：{}", e.1)),
    }
}

impl IntoHashContext for RegisterRequest {
    fn hash_and_to_context(&self) -> ring::digest::Context {
        let mut context = ring::digest::Context::new(&SHA512);
        context.update(&[self.permissions.to_bits()]);
        context.update(self.nonce.as_bytes());
        context.update(self.username.as_bytes());
        context.update(self.password.as_bytes());
        context.update(&self.timestamp.to_be_bytes());
        context
    }
}

#[axum::debug_handler]
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> ApiResponse<RegisterResponse> {
    let challenge = &state.inner.challenge;

    // 验证时间戳
    if let Err(err) = challenge.validate_timestamp(payload.timestamp) {
        return ApiResponse::error(400, format!("时间戳无效：{}", err));
    }

    if let Err(_) = challenge
        .validate(payload.salt, &payload.claim, &payload)
        .await
    {
        return ApiResponse::error(400, "Challenge 挑战失败".to_string());
    }

    match state
        .inner
        .auth_config
        .register(&payload.username, &payload.password)
        .await
    {
        Ok(_user_id) => {
            let permissions: crate::auth::user_store::UserPermissions = payload.permissions.into();
            if let Err(err) = state
                .inner
                .auth_config
                .credentials_store
                .update_permissions(&payload.username, permissions)
                .await
            {
                tracing::warn!("用户 {} 权限更新失败：{}", payload.username, err.1);
            }
            ApiResponse::success(RegisterResponse {
                message: "注册成功".to_string(),
            })
        }
        Err((status, msg)) => ApiResponse::error(status.as_u16() as i32, msg),
    }
}
