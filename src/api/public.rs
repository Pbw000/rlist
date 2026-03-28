//! 公开处理器 - 无需认证即可访问

use axum::{
    Json,
    body::Body,
    extract::{Query, State},
    response::IntoResponse,
};
use ring::digest::SHA512;

use crate::{
    api::{state::AppState, types::*},
    auth::challenge::IntoHashContext,
    storage::model::{FileList, Meta, Storage, UploadInfoParams},
};

use crate::api::types::ApiResponse;

/// 获取上传方式/链接
pub async fn get_upload_info(
    State(state): State<AppState>,
    Json(query): Json<UploadInfoParams>,
) -> impl IntoResponse {
    let registry_guard = state.inner.private_registry.read().await;
    let path = query.path.clone();
    match registry_guard.get_upload_info(query).await {
        Ok(info) => {
            let resp = UploadInfoResponse {
                mode: "direct".to_string(),
                upload_url: info.upload_url,
                method: info.method,
                path,
                form_fields: info.form_fields,
                headers: info.headers,
                complete_url: info.complete_url,
            };
            ApiResponse::success(resp)
        }
        Err(e) => ApiResponse::error(500, e.to_string()),
    }
}

/// 上传文件（Relay 模式）
pub async fn upload_file(
    State(state): State<AppState>,
    Query(query): Query<UploadInfoParams>,
    body: Body,
) -> impl IntoResponse {
    use http_body_util::BodyExt;
    use tokio_util::io::StreamReader;

    let path = query.path.clone();
    let size = query.size;
    let hash = query.hash.clone();

    // 将 Body 转换为 StreamReader 实现 AsyncRead
    let stream = StreamReader::new(
        body.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            .into_data_stream(),
    );

    // 获取注册表并立即释放锁，避免长时间持有锁导致其他请求阻塞
    let result = {
        let registry_guard = state.inner.private_registry.read().await;
        let param = UploadInfoParams {
            path: path.clone(),
            size,
            hash,
        };
        registry_guard.upload_file(&path, stream, param).await
    };

    // 流式上传到存储驱动
    match result {
        Ok(meta) => {
            let resp = UploadResult {
                path,
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
pub async fn complete_upload(
    State(state): State<AppState>,
    Query(query): Query<CompleteUploadParams>,
) -> impl IntoResponse {
    match state
        .complete_upload(
            &query.path,
            &query.upload_id,
            &query.file_id,
            &query.content_hash,
        )
        .await
    {
        Ok(Some(meta)) => {
            let resp = UploadResult {
                path: query.path,
                size: match &meta {
                    Meta::File { size, .. } => *size,
                    Meta::Directory { .. } => 0,
                },
            };
            ApiResponse::success(resp)
        }
        Ok(None) => ApiResponse::success(UploadResult {
            path: query.path,
            size: 0,
        }),
        Err(e) => ApiResponse::error(500, format!("完成上传失败：{}", e)),
    }
}

/// 列出文件和目录
pub async fn list_files(
    State(state): State<AppState>,
    Json(payload): Json<ListQuery>,
) -> impl IntoResponse {
    let path = payload.path.as_deref().unwrap_or("/");
    let registry_guard = state.inner.private_registry.read().await;

    // 使用 cursor 或 page 作为游标，默认第一页（0）
    let cursor = payload.cursor.or_else(|| payload.page.map(|p| p as usize));
    // 默认页大小 20
    let page_size = payload.per_page.unwrap_or(FileList::DEFAULT_PAGE_SIZE);

    match registry_guard.list_files(path, page_size, cursor).await {
        Ok(list) => ApiResponse::success(list),
        Err(e) => ApiResponse::error(500, format!("列出文件失败：{}", e)),
    }
}

/// 获取文件信息
pub async fn get_file_info(
    State(state): State<AppState>,
    Query(query): Query<FsOperation>,
) -> impl IntoResponse {
    let registry_guard = state.inner.private_registry.read().await;

    match registry_guard.get_meta(&query.path).await {
        Ok(meta) => ApiResponse::success(meta),
        Err(e) => ApiResponse::error(404, format!("文件不存在：{}", e)),
    }
}

/// 获取文件下载链接
pub async fn get_file(
    State(state): State<AppState>,
    Query(query): Query<FsOperation>,
) -> impl IntoResponse {
    let registry_guard = state.inner.public_registry.read().await;
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
    let (file_name, size, file_content) = {
        let registry_guard = state.inner.public_registry.read().await;
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

/// 创建目录
pub async fn mkdir(
    State(state): State<AppState>,
    Json(req): Json<FsOperation>,
) -> impl IntoResponse {
    let registry_guard = state.inner.private_registry.read().await;

    match registry_guard.create_folder(&req.path).await {
        Ok(_) => ApiResponse::success(serde_json::json!({"path": req.path})),
        Err(e) => ApiResponse::error(500, format!("创建目录失败：{}", e)),
    }
}

/// 删除文件/目录
pub async fn remove(
    State(state): State<AppState>,
    Json(req): Json<FsOperation>,
) -> impl IntoResponse {
    let registry_guard = state.inner.private_registry.read().await;

    match registry_guard.delete(&req.path).await {
        Ok(_) => ApiResponse::success(serde_json::json!({"path": req.path})),
        Err(e) => ApiResponse::error(500, format!("删除失败：{}", e)),
    }
}

/// 重命名
pub async fn rename(
    State(state): State<AppState>,
    Json(req): Json<RenameRequest>,
) -> impl IntoResponse {
    let registry_guard = state.inner.private_registry.read().await;

    match registry_guard.rename(&req.src_path, &req.new_name).await {
        Ok(_) => ApiResponse::success(
            serde_json::json!({"path": req.src_path, "new_name": req.new_name}),
        ),
        Err(e) => ApiResponse::error(500, format!("重命名失败：{}", e)),
    }
}

/// 复制文件
pub async fn copy(
    State(state): State<AppState>,
    Json(req): Json<MoveCopyRequest>,
) -> impl IntoResponse {
    let registry_guard = state.inner.private_registry.read().await;

    match registry_guard.copy(&req.src_path, &req.dst_path).await {
        Ok(_) => {
            ApiResponse::success(serde_json::json!({"src": req.src_path, "dst": req.dst_path}))
        }
        Err(e) => ApiResponse::error(500, format!("复制失败：{}", e)),
    }
}

/// 移动文件
pub async fn move_file(
    State(state): State<AppState>,
    Json(req): Json<MoveCopyRequest>,
) -> impl IntoResponse {
    let registry_guard = state.inner.private_registry.read().await;

    match registry_guard.move_file(&req.src_path, &req.dst_path).await {
        Ok(_) => {
            ApiResponse::success(serde_json::json!({"src": req.src_path, "dst": req.dst_path}))
        }
        Err(e) => ApiResponse::error(500, format!("移动失败：{}", e)),
    }
}

/// 公开列表文件
pub async fn public_list_files(
    State(state): State<AppState>,
    Json(payload): Json<ListQuery>,
) -> impl IntoResponse {
    let path = payload.path.as_deref().unwrap_or("/");
    let registry_guard = state.get_public_registry().await;

    // 使用 cursor 或 page 作为游标，默认第一页（0）
    let cursor = payload.cursor.or_else(|| payload.page.map(|p| p as usize));
    // 默认页大小 20
    let page_size = payload.per_page.unwrap_or(FileList::DEFAULT_PAGE_SIZE);

    match registry_guard.list_files(path, page_size, cursor).await {
        Ok(list) => ApiResponse::success(list),
        Err(e) => ApiResponse::error(500, format!("列出文件失败：{}", e)),
    }
}

/// 公开下载文件
pub async fn public_download_file(
    State(state): State<AppState>,
    Query(query): Query<FsOperation>,
) -> impl IntoResponse {
    let registry_guard = state.get_public_registry().await;

    match registry_guard.get_download_meta_by_path(&query.path).await {
        Ok(meta) => ApiResponse::success(meta),
        Err(e) => ApiResponse::error(404, format!("获取下载链接失败：{}", e)),
    }
}

/// 获取 Challenge
#[axum::debug_handler]
pub async fn get_challenge(State(state): State<AppState>) -> ApiResponse<ChallengeResponse> {
    let salt_value = state.inner.challenge.challenge.get_current_salt();
    ApiResponse::success(ChallengeResponse { salt: salt_value })
}

/// 用户登录
#[axum::debug_handler]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> ApiResponse<LoginResponse> {
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
        .login(payload.username, payload.password)
        .await
    {
        Ok(token) => ApiResponse::success(LoginResponse { token }),
        Err((status, msg)) => ApiResponse::error(status.as_u16() as i32, msg),
    }
}

impl IntoHashContext for LoginRequest {
    fn hash_and_to_context(&self) -> ring::digest::Context {
        let mut context = ring::digest::Context::new(&SHA512);
        context.update(self.nonce.as_bytes());
        context.update(self.username.as_bytes());
        context.update(self.password.as_bytes());
        context.update(&self.timestamp.to_be_bytes());
        context
    }
}
