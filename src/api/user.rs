//! 用户处理器 - 需要认证访问

use axum::extract::Query;
use axum::{extract::State, response::IntoResponse};

use crate::api::{state::AppState, types::*};
use crate::{Meta, Storage};

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
    let pub_storage = state.list_public_storages().await;
    let pri_storage = state.list_private_storages().await;
    let storages = serde_json::json!({
        "public": pub_storage,
        "private": pri_storage,
    });
    ApiResponse::success(storages)
}
/// 获取文件下载链接
pub async fn get_file(
    State(state): State<AppState>,
    Query(query): Query<FsOperation>,
) -> impl IntoResponse {
    let registry_guard = state.inner.private_registry.read().await;
    match registry_guard.get_download_meta_by_path(&query.path).await {
        Ok(meta) => {
            // 使用存储驱动自己生成的下载 URL
            let resp = FileResponse {
                name: query
                    .path
                    .split('/')
                    .last()
                    .unwrap_or("unknown")
                    .to_string(),
                url: meta.download_url,
                size: meta.size,
                hash: meta.hash,
            };
            ApiResponse::success(resp)
        }
        Err(e) => ApiResponse::error(404, format!("获取下载链接失败：{}", e)),
    }
}

/// 下载文件 - 直接流式传输文件内容
pub async fn download_file(
    State(state): State<AppState>,
    Query(query): Query<FsOperation>,
) -> impl IntoResponse {
    use axum::body::Body;
    use axum::response::Response;
    use futures_util::stream::StreamExt;
    use tokio_util::io::ReaderStream;

    // 先获取文件元数据（短暂持有锁）
    let (file_name, size, file_content) = {
        let registry_guard = state.inner.private_registry.read().await;

        let meta = match registry_guard.get_meta(&query.path).await {
            Ok(m) => m,
            Err(e) => {
                return ApiResponse::<()>::error(404, format!("文件不存在：{}", e)).into_response();
            }
        };

        let (file_name, size) = match &meta {
            Meta::File { name, size, .. } => (name.clone(), *size),
            Meta::Directory { .. } => {
                return ApiResponse::<()>::error(400, "不能下载目录".to_string()).into_response();
            }
        };

        // 下载文件流
        match registry_guard.download_file(&query.path).await {
            Ok(file_content) => (file_name, size, file_content),
            Err(e) => {
                return ApiResponse::<()>::error(500, format!("下载失败：{}", e)).into_response();
            }
        }
    };
    // 锁在这里已经释放

    // 创建流式响应
    let stream = ReaderStream::new(file_content);

    let body = Body::from_stream(stream.filter_map(|result| async move {
        match result {
            Ok(bytes) => Some(Ok::<axum::body::Bytes, std::convert::Infallible>(
                axum::body::Bytes::from(bytes),
            )),
            Err(_) => None,
        }
    }));

    let response = Response::builder()
        .header("Content-Type", "application/octet-stream")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", file_name),
        )
        .header("Content-Length", size.to_string())
        .body(body)
        .map_err(|e| format!("构建响应失败：{}", e));

    match response {
        Ok(resp) => resp.into_response(),
        Err(e) => ApiResponse::<()>::error(500, e).into_response(),
    }
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
