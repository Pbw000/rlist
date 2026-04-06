//! 用户处理器 - 需要认证访问

use axum::body::Body;
use axum::extract::Extension;
use axum::extract::Json;
use axum::extract::Query;
use axum::{extract::State, response::IntoResponse};
use std::borrow::Cow;

use crate::api::{state::AppState, types::*};
use crate::storage::model::{FileList, UploadInfoParams};
use crate::{Meta, Storage};

use crate::api::types::ApiResponse;
use crate::auth::challenge::IntoHashContext;
use ring::digest::{Context, SHA512};

impl IntoHashContext for LoginRequest {
    fn hash_and_to_context(&self) -> Context {
        let mut context = Context::new(&SHA512);
        context.update(self.nonce.as_bytes());
        context.update(self.username.as_bytes());
        context.update(self.password.as_bytes());
        context.update(&self.timestamp.to_be_bytes());
        context
    }
}

/// 获取 Challenge
pub async fn get_challenge(State(state): State<AppState>) -> impl IntoResponse {
    let salt = state.inner.challenge.challenge.get_current_salt();
    ApiResponse::success(serde_json::json!({
        "salt": salt,
    }))
}

/// 用户登录
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    let challenge = &state.inner.challenge;

    if let Err(err) = challenge.validate_timestamp(payload.timestamp) {
        return ApiResponse::error(400, format!("时间戳无效：{}", err));
    }

    if let Err(_) = challenge
        .validate::<_, 4>(payload.salt, &payload.claim, &payload)
        .await
    {
        return ApiResponse::error(400, "Challenge 挑战失败".to_string());
    }

    match state
        .inner
        .auth_config
        .login(payload.username, payload.password)
        .await
    {
        Ok(token) => ApiResponse::success(LoginResponse { token }),
        Err((status, msg)) => ApiResponse::error(status.as_u16() as i32, msg),
    }
}

/// 应用用户根目录前缀（如果有）
/// 使用 Cow 避免不必要的字符串分配
#[inline]
fn apply_root_dir<'a>(path: &'a str, root_dir: Option<&'a Extension<String>>) -> Cow<'a, str> {
    if let Some(Extension(root)) = root_dir {
        let path_trimmed = path.trim_start_matches('/');
        if path_trimmed.is_empty() {
            Cow::Owned(root.trim_end_matches('/').to_string())
        } else {
            Cow::Owned(format!("{}/{}", root.trim_end_matches('/'), path_trimmed))
        }
    } else {
        Cow::Borrowed(path)
    }
}

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

/// 刷新路径缓存
#[axum::debug_handler]
pub async fn refresh_cache(
    State(state): State<AppState>,
    root_dir: Option<Extension<String>>,
    Json(req): Json<RefreshCacheRequest>,
) -> impl IntoResponse {
    let full_path = apply_root_dir(&req.path, root_dir.as_ref());
    match state.build_cache(&full_path).await {
        Ok(()) => ApiResponse::success(serde_json::json!({
            "message": format!("缓存刷新成功：{}", req.path)
        })),
        Err(e) => ApiResponse::error(500, format!("刷新缓存失败：{}", e)),
    }
}

/// 获取文件下载链接
pub async fn get_file(
    State(state): State<AppState>,
    root_dir: Option<Extension<String>>,
    Query(query): Query<FsOperation>,
) -> impl IntoResponse {
    let full_path = apply_root_dir(&query.path, root_dir.as_ref());
    let registry_guard = state.inner.private_registry.read().await;
    match registry_guard.get_download_meta_by_path(&full_path).await {
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
    root_dir: Option<Extension<String>>,
    Query(query): Query<FsOperation>,
) -> impl IntoResponse {
    use axum::body::Body;
    use axum::response::Response;
    use futures_util::stream::StreamExt;
    use tokio_util::io::ReaderStream;

    let full_path = apply_root_dir(&query.path, root_dir.as_ref());

    // 先获取文件元数据（短暂持有锁）
    let (file_name, size, file_content) = {
        let registry_guard = state.inner.private_registry.read().await;

        let meta = match registry_guard.get_meta(&full_path).await {
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
        match registry_guard.download_file(&full_path).await {
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

// ==================== 需要认证的文件操作 API ====================

/// 列出文件和目录
#[axum::debug_handler]
pub async fn list_files(
    State(state): State<AppState>,
    root_dir: Option<Extension<String>>,
    Json(payload): Json<ListQuery>,
) -> impl IntoResponse {
    // 触发刷新通知
    state.trigger_refresh();

    let path = payload.path.as_deref().unwrap_or("/");
    let full_path = apply_root_dir(path, root_dir.as_ref());
    let registry_guard = state.inner.private_registry.read().await;

    let cursor = payload.cursor.or_else(|| payload.page.map(|p| p as usize));
    let page_size = payload.per_page.unwrap_or(FileList::DEFAULT_PAGE_SIZE);

    match registry_guard
        .list_files(&full_path, page_size, cursor)
        .await
    {
        Ok(list) => ApiResponse::success(list),
        Err(e) => ApiResponse::error(500, format!("列出文件失败：{}", e)),
    }
}

/// 获取文件信息
#[axum::debug_handler]
pub async fn get_file_info(
    State(state): State<AppState>,
    root_dir: Option<Extension<String>>,
    Query(query): Query<FsOperation>,
) -> impl IntoResponse {
    let full_path = apply_root_dir(&query.path, root_dir.as_ref());
    let registry_guard = state.inner.private_registry.read().await;

    match registry_guard.get_meta(&full_path).await {
        Ok(meta) => ApiResponse::success(meta),
        Err(e) => ApiResponse::error(404, format!("文件不存在：{}", e)),
    }
}

/// 获取上传方式/链接
#[axum::debug_handler]
pub async fn get_upload_info(
    State(state): State<AppState>,
    root_dir: Option<Extension<String>>,
    Json(query): Json<UploadInfoParams>,
) -> impl IntoResponse {
    let full_path = apply_root_dir(&query.path, root_dir.as_ref());

    let query_with_prefix = UploadInfoParams {
        path: full_path.into_owned(),
        size: query.size,
        hash: query.hash.clone(),
    };

    let registry_guard = state.inner.private_registry.read().await;
    match registry_guard.get_upload_info(query_with_prefix).await {
        Ok(info) => {
            let resp = UploadInfoResponse {
                mode: "direct".into(),
                upload_url: info.upload_url,
                method: info.method,
                path: query.path,
                form_fields: info.form_fields,
                headers: info.headers,
                complete_params: info.complete_params,
            };
            ApiResponse::success(resp)
        }
        Err(e) => ApiResponse::error(500, e.to_string()),
    }
}

/// 上传文件（Relay 模式）
#[axum::debug_handler]
pub async fn upload_file(
    State(state): State<AppState>,
    root_dir: Option<Extension<String>>,
    Query(query): Query<UploadInfoParams>,
    body: Body,
) -> impl IntoResponse {
    use http_body_util::BodyExt;
    use tokio_util::io::StreamReader;

    let full_path = apply_root_dir(&query.path, root_dir.as_ref());
    let size = query.size;
    let hash = query.hash.clone();

    let stream = StreamReader::new(
        body.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            .into_data_stream(),
    );

    let result = {
        let registry_guard = state.inner.private_registry.read().await;
        let param = UploadInfoParams {
            path: full_path.clone().into_owned(),
            size,
            hash,
        };
        registry_guard.upload_file(&full_path, stream, param).await
    };

    match result {
        Ok(meta) => {
            let resp = UploadResult {
                path: query.path,
                size: match &meta {
                    Meta::File { size, .. } => *size,
                    Meta::Directory { .. } => 0,
                },
            };
            ApiResponse::success(resp)
        }
        Err(e) => ApiResponse::error(500, format!("上传失败：{}", e)),
    }
}

/// 完成上传（Direct 模式）
#[axum::debug_handler]
pub async fn complete_upload(
    State(state): State<AppState>,
    root_dir: Option<Extension<String>>,
    Json(req): Json<CompleteUploadRequest>,
) -> impl IntoResponse {
    let full_path = apply_root_dir(&req.path, root_dir.as_ref());

    match state
        .complete_upload(
            &full_path,
            &req.info.upload_id,
            &req.info.file_id,
            &req.info.content_hash,
        )
        .await
    {
        Ok(Some(meta)) => {
            let resp = CompleteUploadResponse {
                path: req.path,
                size: match &meta {
                    Meta::File { size, .. } => *size,
                    Meta::Directory { .. } => 0,
                },
            };
            ApiResponse::success(resp)
        }
        Ok(None) => ApiResponse::success(CompleteUploadResponse {
            path: req.path,
            size: 0,
        }),
        Err(e) => ApiResponse::error(500, format!("完成上传失败：{}", e)),
    }
}

/// 创建目录
#[axum::debug_handler]
pub async fn mkdir(
    State(state): State<AppState>,
    root_dir: Option<Extension<String>>,
    Json(req): Json<FsOperation>,
) -> impl IntoResponse {
    let full_path = apply_root_dir(&req.path, root_dir.as_ref());
    let registry_guard = state.inner.private_registry.read().await;

    match registry_guard.create_folder(&full_path).await {
        Ok(_) => ApiResponse::success(serde_json::json!({"path": req.path})),
        Err(e) => ApiResponse::error(500, format!("创建目录失败：{}", e)),
    }
}

/// 删除文件/目录
#[axum::debug_handler]
pub async fn remove(
    State(state): State<AppState>,
    root_dir: Option<Extension<String>>,
    Json(req): Json<FsOperation>,
) -> impl IntoResponse {
    let full_path = apply_root_dir(&req.path, root_dir.as_ref());
    let registry_guard = state.inner.private_registry.read().await;

    match registry_guard.delete(&full_path).await {
        Ok(_) => ApiResponse::success(serde_json::json!({"path": req.path})),
        Err(e) => ApiResponse::error(500, format!("删除失败：{}", e)),
    }
}

/// 重命名
#[axum::debug_handler]
pub async fn rename(
    State(state): State<AppState>,
    root_dir: Option<Extension<String>>,
    Json(req): Json<RenameRequest>,
) -> impl IntoResponse {
    let full_src_path = apply_root_dir(&req.src_path, root_dir.as_ref());
    let registry_guard = state.inner.private_registry.read().await;

    match registry_guard.rename(&full_src_path, &req.new_name).await {
        Ok(_) => ApiResponse::success(
            serde_json::json!({"path": req.src_path, "new_name": req.new_name}),
        ),
        Err(e) => ApiResponse::error(500, format!("重命名失败：{}", e)),
    }
}

/// 复制文件
#[axum::debug_handler]
pub async fn copy(
    State(state): State<AppState>,
    root_dir: Option<Extension<String>>,
    Json(req): Json<MoveCopyRequest>,
) -> impl IntoResponse {
    let full_src_path = apply_root_dir(&req.src_path, root_dir.as_ref());
    let full_dst_path = apply_root_dir(&req.dst_path, root_dir.as_ref());
    let registry_guard = state.inner.private_registry.read().await;

    match registry_guard.copy(&full_src_path, &full_dst_path).await {
        Ok(_) => {
            ApiResponse::success(serde_json::json!({"src": req.src_path, "dst": req.dst_path}))
        }
        Err(e) => ApiResponse::error(500, format!("复制失败：{}", e)),
    }
}

/// 移动文件
#[axum::debug_handler]
pub async fn move_file(
    State(state): State<AppState>,
    root_dir: Option<Extension<String>>,
    Json(req): Json<MoveCopyRequest>,
) -> impl IntoResponse {
    let full_src_path = apply_root_dir(&req.src_path, root_dir.as_ref());
    let full_dst_path = apply_root_dir(&req.dst_path, root_dir.as_ref());
    let registry_guard = state.inner.private_registry.read().await;

    match registry_guard
        .move_file(&full_src_path, &full_dst_path)
        .await
    {
        Ok(_) => {
            ApiResponse::success(serde_json::json!({"src": req.src_path, "dst": req.dst_path}))
        }
        Err(e) => ApiResponse::error(500, format!("移动失败：{}", e)),
    }
}
