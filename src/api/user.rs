//! 用户处理器 - 需要认证访问

use axum::{extract::State, response::IntoResponse};

use crate::api::{state::AppState, types::*};

use crate::api::types::ApiResponse;

/// 获取当前用户信息
#[axum::debug_handler]
pub async fn get_current_user() -> ApiResponse<serde_json::Value> {
    ApiResponse::success(serde_json::json!({
        "message": "已认证"
    }))
}

/// 列出所有存储
pub async fn list_storages(State(state): State<AppState>) -> impl IntoResponse {
    let names = state.list_storages().await;
    let storages: Vec<StorageInfo> = names
        .into_iter()
        .map(|name| StorageInfo {
            id: name.clone(),
            name,
            driver: "unknown".to_string(),
            status: "work".to_string(),
        })
        .collect();

    ApiResponse::success(storages)
}

impl From<crate::auth::user_store::UserPermissions> for UserPermissionsResponse {
    fn from(perms: crate::auth::user_store::UserPermissions) -> Self {
        UserPermissionsResponse {
            read: perms.read,
            download: perms.download,
            upload: perms.upload,
            delete: perms.delete,
            move_obj: perms.move_obj,
            copy: perms.copy,
            create_dir: perms.create_dir,
            list: perms.list,
        }
    }
}
