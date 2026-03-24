//! API 请求处理器
use axum::{
    Json,
    body::Body,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Deserializer, Serialize};

use crate::storage::model::{Meta, Storage};
use crate::{api::state::AppState, storage::model::UploadInfoParams};

/// 反序列化 u64，支持字符串或数字格式
fn deserialize_u64_from_str_or_num<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNum<T> {
        String(String),
        Num(T),
    }
    match StringOrNum::deserialize(deserializer)? {
        StringOrNum::String(s) => s.parse::<u64>().map_err(Error::custom),
        StringOrNum::Num(n) => Ok(n),
    }
}

/// 列出文件和目录请求参数
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub path: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub storage: Option<String>,
}

/// 文件操作请求
#[derive(Debug, Deserialize)]
pub struct FsOperation {
    pub path: String,
}

/// 重命名请求
#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    pub src_path: String,
    pub new_name: String,
}

/// 移动/复制请求
#[derive(Debug, Deserialize)]
pub struct MoveCopyRequest {
    pub src_path: String,
    pub dst_path: String,
}

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

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

/// 上传文件响应
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum UploadResponse {
    /// Direct 模式：返回上传信息
    Direct(UploadInfoResponse),
    /// Relay 模式：直接上传成功
    Relay { path: String },
}

#[derive(Debug, Serialize)]
pub struct UploadInfoResponse {
    pub mode: String,
    pub upload_url: String,
    pub method: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form_fields: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complete_url: Option<String>,
}

/// 完成上传请求参数（Direct 模式）
#[derive(Debug, Deserialize)]
pub struct CompleteUploadParams {
    pub path: String,
    pub upload_id: String,
    pub file_id: String,
    pub content_hash: String,
}

/// 获取上传方式/链接
pub async fn get_upload_info(
    State(state): State<AppState>,
    Json(query): Json<UploadInfoParams>,
) -> impl IntoResponse {
    let registry_guard = state.inner.registry.read().await;
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
        let registry_guard = state.inner.registry.read().await;
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

/// 上传结果响应
#[derive(Debug, Serialize)]
pub struct UploadResult {
    pub path: String,
    pub size: u64,
}

/// 完成上传（Direct 模式）
pub async fn complete_upload(
    State(state): State<AppState>,
    Query(query): Query<CompleteUploadParams>,
) -> impl IntoResponse {
    // 调用存储驱动的 complete_upload 方法
    // path 已经是绝对路径（包含存储前缀）
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
        Ok(None) => {
            // 返回 None 表示不需要 complete 步骤（默认实现）
            ApiResponse::success(UploadResult {
                path: query.path,
                size: 0,
            })
        }
        Err(e) => ApiResponse::error(500, format!("完成上传失败：{}", e)),
    }
}

/// 存储配置请求
#[derive(Debug, Deserialize)]
pub struct StorageConfigRequest {
    pub name: String,
    pub driver: String,
    pub config: serde_json::Value,
}

// ==================== 文件系统接口 ====================

/// 列出文件和目录
pub async fn list_files(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> impl IntoResponse {
    let path = query.path.as_deref().unwrap_or("/");
    let registry_guard = state.inner.registry.read().await;

    match registry_guard
        .list_files(path, query.per_page.unwrap_or(100), None)
        .await
    {
        Ok(list) => ApiResponse::success(list),
        Err(e) => ApiResponse::error(500, format!("列出文件失败：{}", e)),
    }
}

/// 获取文件信息
pub async fn get_file_info(
    State(state): State<AppState>,
    Query(query): Query<FsOperation>,
) -> impl IntoResponse {
    let registry_guard = state.inner.registry.read().await;

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
    let registry_guard = state.inner.registry.read().await;
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
        let registry_guard = state.inner.registry.read().await;

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
    let registry_guard = state.inner.registry.read().await;

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
    let registry_guard = state.inner.registry.read().await;

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
    let registry_guard = state.inner.registry.read().await;

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
    let registry_guard = state.inner.registry.read().await;

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
    let registry_guard = state.inner.registry.read().await;

    match registry_guard.move_(&req.src_path, &req.dst_path).await {
        Ok(_) => {
            ApiResponse::success(serde_json::json!({"src": req.src_path, "dst": req.dst_path}))
        }
        Err(e) => ApiResponse::error(500, format!("移动失败：{}", e)),
    }
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

/// 添加存储
pub async fn add_storage(
    State(_state): State<AppState>,
    Json(_req): Json<StorageConfigRequest>,
) -> impl IntoResponse {
    ApiResponse::success(serde_json::json!({"message": "存储添加功能开发中"}))
}

/// 删除存储
pub async fn remove_storage(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    state.remove_storage(&name).await;
    ApiResponse::success(serde_json::json!({"deleted": name}))
}

#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub file_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileResponse {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub hash: String,
}

#[derive(Debug, Serialize)]
pub struct StorageInfo {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub status: String,
}

/// 注册请求
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    #[serde(deserialize_with = "deserialize_u64_from_str_or_num")]
    pub salt: u64,
    #[serde(deserialize_with = "deserialize_u64_from_str_or_num")]
    pub timestamp: u64,
    #[serde(deserialize_with = "deserialize_u64_from_str_or_num")]
    pub nonce: u64,
    pub claim: String,
}

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    #[serde(deserialize_with = "deserialize_u64_from_str_or_num")]
    pub salt: u64,
    #[serde(deserialize_with = "deserialize_u64_from_str_or_num")]
    pub timestamp: u64,
    #[serde(deserialize_with = "deserialize_u64_from_str_or_num")]
    pub nonce: u64,
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

    // 验证 challenge (payload = timestamp + username + password + nonce)
    let challenge_payload = format!(
        "{}{}{}{}",
        payload.timestamp, payload.username, payload.password, payload.nonce
    );
    if let Err(_) = challenge
        .validate(payload.salt, &payload.claim, &challenge_payload)
        .await
    {
        return ApiResponse::error(400, "Challenge 验证失败".to_string());
    }

    match state
        .inner
        .auth_config
        .register(payload.username, payload.password)
        .await
    {
        Ok(_user_id) => ApiResponse::success(RegisterResponse {
            message: "注册成功".to_string(),
        }),
        Err((status, msg)) => ApiResponse::error(status.as_u16() as i32, msg),
    }
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

    // 验证 challenge (payload = timestamp + username + password + nonce)
    let challenge_payload = format!(
        "{}{}{}{}",
        payload.timestamp, payload.username, payload.password, payload.nonce
    );
    if let Err(_) = challenge
        .validate(payload.salt, &payload.claim, &challenge_payload)
        .await
    {
        return ApiResponse::error(400, "Challenge 验证失败".to_string());
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

/// 获取当前用户信息
#[axum::debug_handler]
pub async fn get_current_user() -> ApiResponse<serde_json::Value> {
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
#[axum::debug_handler]
pub async fn get_challenge(State(state): State<AppState>) -> ApiResponse<ChallengeResponse> {
    let salt_value = state.inner.challenge.challenge.get_current_salt();
    ApiResponse::success(ChallengeResponse { salt: salt_value })
}

pub async fn public_list_files(
    State(state): State<AppState>,
    Json(payload): Json<ListQuery>,
) -> impl IntoResponse {
    let path = payload.path.as_deref().unwrap_or("/");
    let registry_guard = state.get_public_registry().await;

    match registry_guard
        .list_files(path, payload.per_page.unwrap_or(100), None)
        .await
    {
        Ok(list) => ApiResponse::success(list),
        Err(e) => ApiResponse::error(500, format!("列出文件失败：{}", e)),
    }
}

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
