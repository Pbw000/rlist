//! API 请求处理器
use axum::{
    Json,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::api::state::AppState;
use crate::storage::model::{Meta, Storage};

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

/// 文件列表请求参数
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

/// 路径导航请求
#[derive(Debug, Deserialize)]
pub struct PathNavigateRequest {
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

/// 上传请求参数
#[derive(Debug, Deserialize)]
pub struct UploadQuery {
    pub path: String,
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
}

/// 获取上传方式/链接
pub async fn get_upload_info(
    State(state): State<AppState>,
    Query(query): Query<UploadQuery>,
) -> impl IntoResponse {
    let registry_guard = state.inner.registry.read().await;

    // 获取文件元数据（如果文件已存在，获取其大小）
    let size = match registry_guard.get_meta(&query.path).await {
        Ok(meta) => match meta {
            Meta::File { size, .. } => size,
            Meta::Directory { .. } => 0,
        },
        Err(_) => 0,
    };

    match registry_guard.get_upload_info(&query.path, size).await {
        Ok(info) => {
            let resp = UploadInfoResponse {
                mode: "direct".to_string(),
                upload_url: info.upload_url,
                method: info.method,
                path: info.path,
                form_fields: info.form_fields,
            };
            ApiResponse::success(UploadResponse::Direct(resp))
        }
        Err(_) => {
            // 如果不支持 Direct 模式，返回 Relay 模式指示
            let resp = UploadResponse::Relay {
                path: query.path.clone(),
            };
            ApiResponse::success(resp)
        }
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
        Ok(list) => {
            let resp = ListResponse {
                content: list
                    .items
                    .into_iter()
                    .map(|m| meta_to_file_info(m, path))
                    .collect(),
                total: list.total as usize,
                read_me: None,
            };
            ApiResponse::success(resp)
        }
        Err(e) => ApiResponse::error(500, format!("列出文件失败：{}", e)),
    }
}

/// 路径导航 - 支持浏览器文件路径导航
pub async fn navigate_path(
    State(state): State<AppState>,
    Json(req): Json<PathNavigateRequest>,
) -> impl IntoResponse {
    let path = &req.path;
    let registry_guard = state.inner.registry.read().await;

    // 构建路径层级
    let mut breadcrumbs: Vec<NavBreadcrumb> = Vec::new();

    // 添加根目录
    breadcrumbs.push(NavBreadcrumb {
        name: "根目录".to_string(),
        path: "/".to_string(),
        is_current: path == "/",
    });

    // 解析路径层级
    if path != "/" && !path.is_empty() {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let mut current_path = String::new();

        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }

            if current_path.is_empty() {
                current_path = format!("/{}", part);
            } else {
                current_path = format!("{}/{}", current_path, part);
            }

            breadcrumbs.push(NavBreadcrumb {
                name: part.to_string(),
                path: current_path.clone(),
                is_current: i == parts.len() - 1,
            });
        }
    }

    // 获取当前路径的文件列表
    let content = match registry_guard.list_files(path, 100, None).await {
        Ok(list) => list
            .items
            .into_iter()
            .map(|m| meta_to_file_info(m, path))
            .collect(),
        Err(_) => Vec::new(),
    };

    let resp = NavResponse {
        breadcrumbs,
        content,
        current_path: path.clone(),
    };

    ApiResponse::success(resp)
}

/// 获取父目录列表 - 用于选择目标路径
pub async fn get_parent_dirs(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> impl IntoResponse {
    let path = query.path.as_deref().unwrap_or("/");
    let registry_guard = state.inner.registry.read().await;

    // 构建父目录列表
    let mut parent_dirs: Vec<DirInfo> = Vec::new();

    // 添加根目录
    parent_dirs.push(DirInfo {
        name: "根目录".to_string(),
        path: "/".to_string(),
    });

    // 解析当前路径的父目录
    if path != "/" && !path.is_empty() {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let mut current_path = String::new();

        for part in parts.iter() {
            if part.is_empty() {
                continue;
            }

            if current_path.is_empty() {
                current_path = format!("/{}", part);
            } else {
                current_path = format!("{}/{}", current_path, part);
            }

            // 只添加到当前路径的父目录，不包括当前路径本身
            if current_path != path {
                parent_dirs.push(DirInfo {
                    name: part.to_string(),
                    path: current_path.clone(),
                });
            }
        }
    }

    // 获取当前目录下的子目录（用于选择子目录作为目标）
    if let Ok(list) = registry_guard.list_files(path, 100, None).await {
        for item in list.items {
            if let Meta::Directory { name, .. } = item {
                let item_path = if path.ends_with('/') || path == "/" {
                    format!("{}{}", path, name)
                } else {
                    format!("{}/{}", path, name)
                };
                parent_dirs.push(DirInfo {
                    name: name.clone(),
                    path: item_path,
                });
            }
        }
    }

    ApiResponse::success(parent_dirs)
}

/// 获取文件信息
pub async fn get_file_info(
    State(state): State<AppState>,
    Query(query): Query<FsOperation>,
) -> impl IntoResponse {
    let registry_guard = state.inner.registry.read().await;

    match registry_guard.get_meta(&query.path).await {
        Ok(meta) => {
            let file_info = meta_to_file_info(meta, &query.path);
            ApiResponse::success(file_info)
        }
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

    let registry_guard = state.inner.registry.read().await;

    // 获取文件元数据以确定文件名和大小
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
        Ok(file_content) => {
            // 获取文件名（从路径中提取）
            let file_name = file_name.clone();

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
        Err(e) => ApiResponse::<()>::error(500, format!("下载失败：{}", e)).into_response(),
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

/// 上传文件
pub async fn upload_file(
    State(state): State<AppState>,
    Query(query): Query<UploadQuery>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let registry_guard = state.inner.registry.read().await;

    match registry_guard.upload_file(&query.path, body.to_vec()).await {
        Ok(_) => ApiResponse::success(serde_json::json!({"path": query.path})),
        Err(e) => ApiResponse::error(500, format!("上传失败：{}", e)),
    }
}

// ==================== 存储管理接口 ====================

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

// ==================== 辅助函数 ====================

/// 将 Meta 转换为 FileInfo
fn meta_to_file_info(meta: Meta, parent_path: &str) -> FileInfo {
    let name = match &meta {
        Meta::File { name, .. } | Meta::Directory { name, .. } => name.clone(),
    };

    let path = if parent_path.ends_with('/') || parent_path == "/" {
        format!("{}{}", parent_path, name)
    } else {
        format!("{}/{}", parent_path, name)
    };

    let (size, file_type, modified) = match meta {
        Meta::File {
            size, modified_at, ..
        } => (
            size,
            "file".to_string(),
            modified_at.map(|dt| dt.to_rfc3339()),
        ),
        Meta::Directory { modified_at, .. } => {
            (0, "dir".to_string(), modified_at.map(|dt| dt.to_rfc3339()))
        }
    };

    FileInfo {
        name,
        path,
        size,
        file_type,
        modified,
    }
}
#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub content: Vec<FileInfo>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_me: Option<String>,
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

// ==================== 路径导航响应类型 ====================

/// 导航面包屑
#[derive(Debug, Serialize)]
pub struct NavBreadcrumb {
    pub name: String,
    pub path: String,
    pub is_current: bool,
}

/// 目录信息（用于选择目标路径）
#[derive(Debug, Serialize)]
pub struct DirInfo {
    pub name: String,
    pub path: String,
}

/// 路径导航响应
#[derive(Debug, Serialize)]
pub struct NavResponse {
    pub breadcrumbs: Vec<NavBreadcrumb>,
    pub content: Vec<FileInfo>,
    pub current_path: String,
}
