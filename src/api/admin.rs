//! 管理员处理器 - 需要管理员权限

use crate::{
    Storage,
    api::{state::AppState, types::*},
    auth::challenge::IntoHashContext,
    storage::all::AllDriver,
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use ring::digest::SHA512;

use crate::api::types::ApiResponse;
use crate::storage::all::AllDriverConfigMeta;

/// 获取可用的存储驱动列表
pub async fn get_storage_drivers() -> impl IntoResponse {
    let drivers = AllDriverConfigMeta::all_drivers();
    ApiResponse::success(drivers)
}

/// 获取存储驱动配置模板
pub async fn get_storage_template(Path(driver): Path<String>) -> impl IntoResponse {
    // 解析驱动类型
    let driver_type: AllDriverConfigMeta = match driver.parse() {
        Ok(d) => d,
        Err(_) => return ApiResponse::error(404, format!("未知的存储驱动：{}", driver)),
    };

    // 使用宏生成的辅助方法获取模板
    let template = driver_type.get_template_json();

    ApiResponse::success(StorageTemplateResponse {
        driver: driver_type.driver_name().to_string(),
        template,
    })
}

/// 添加存储
pub async fn add_storage(
    State(state): State<AppState>,
    Json(req): Json<AddStorageRequest>,
) -> impl IntoResponse {
    // 验证 prefix 格式
    let prefix = req.prefix.trim_start_matches('/');
    if prefix.is_empty() {
        return ApiResponse::error(400, "存储前缀不能为空".to_string());
    }

    // 保存驱动名称
    let driver_name = req.driver.driver_name().to_string();

    // 直接使用请求中的 driver ConfigMeta
    let config_meta = req.driver;

    // 使用 from_auth_data 创建存储实例
    let storage = match AllDriver::from_auth_data(config_meta) {
        Ok(s) => s,
        Err(e) => return ApiResponse::error(400, format!("初始化存储失败：{}", e)),
    };

    // 添加到注册表
    if req.public.unwrap_or(false) {
        state.add_public_storage(prefix, storage).await;
    } else {
        state.add_storage(prefix, storage).await;
    }

    // 构建缓存
    if let Err(e) = state.build_cache(&format!("/{}", prefix)).await {
        tracing::warn!("构建存储缓存失败：{}", e);
    }

    ApiResponse::success(AddStorageResponse {
        prefix: format!("/{}", prefix),
        driver: driver_name,
        message: "存储添加成功".to_string(),
    })
}

pub async fn remove_pub_storage(
    State(state): State<AppState>,
    Path(index): Path<usize>,
) -> impl IntoResponse {
    match state.remove_public_storage(index).await {
        Some(name) => ApiResponse::success(serde_json::json!({"deleted": name})),
        None => ApiResponse::error(404, "Storage not found".to_string()),
    }
}
pub async fn remove_private_storage(
    State(state): State<AppState>,
    Path(index): Path<usize>,
) -> impl IntoResponse {
    match state.remove_private_storage(index).await {
        Some(name) => ApiResponse::success(serde_json::json!({"deleted": name})),
        None => ApiResponse::error(404, "Storage not found".to_string()),
    }
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
