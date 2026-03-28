use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Deserializer, Serialize};

use crate::auth::user_store::UserPermissions;

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

// ==================== 请求类型 ====================

/// 列出文件和目录请求参数
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub path: Option<String>,
    /// 页码（从 0 开始），与 cursor 二选一
    pub page: Option<u32>,
    /// 游标（页码，从 0 开始），与 page 二选一
    pub cursor: Option<usize>,
    /// 每页数量，默认 20
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

/// 删除用户请求
#[derive(Debug, Deserialize)]
pub struct RmUserRequest {
    pub user_name: String,
}

/// 修改用户权限请求
#[derive(Debug, Deserialize)]
pub struct UpdatePermissionsRequest {
    pub user_name: String,
    pub permissions: UserPermissions,
}

/// 更新用户根目录请求
#[derive(Debug, Deserialize)]
pub struct UpdateUserRootDirRequest {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_dir: Option<String>,
}

/// 移动/复制请求
#[derive(Debug, Deserialize)]
pub struct MoveCopyRequest {
    pub src_path: String,
    pub dst_path: String,
}

/// 存储配置请求
#[derive(Debug, Deserialize)]
pub struct StorageConfigRequest {
    pub name: String,
    pub driver: String,
    pub config: serde_json::Value,
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
    pub nonce: String,
    pub claim: String,
    pub permissions: UserPermissions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_dir: Option<String>,
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
    pub nonce: String,
    pub claim: String,
}

/// 完成上传请求参数（Direct 模式）
#[derive(Debug, Deserialize)]
pub struct CompleteUploadParams {
    pub path: String,
    pub upload_id: String,
    pub file_id: String,
    pub content_hash: crate::storage::model::Hash,
}

/// 刷新缓存请求
#[derive(Debug, Deserialize)]
pub struct RefreshCacheRequest {
    pub path: String,
}

// ==================== 响应类型 ====================

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
    fn into_response(self) -> axum::response::Response {
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

/// 上传结果响应
#[derive(Debug, Serialize)]
pub struct UploadResult {
    pub path: String,
    pub size: u64,
}

/// 用户信息响应
#[derive(Debug, Serialize)]
pub struct UserInfoResponse {
    pub username: String,
    pub permissions: UserPermissionsResponse,
}

/// 用户权限响应
#[derive(Debug, Serialize)]
pub struct UserPermissionsResponse {
    pub read: bool,
    pub download: bool,
    pub upload: bool,
    pub delete: bool,
    pub move_obj: bool,
    pub copy: bool,
    pub create_dir: bool,
    pub list: bool,
}

/// 文件信息响应
#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub file_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
}

/// 文件下载响应
#[derive(Debug, Serialize)]
pub struct FileResponse {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub hash: crate::storage::model::Hash,
}

/// 存储信息响应
#[derive(Debug, Serialize)]
pub struct StorageInfo {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub status: String,
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

/// Challenge 响应
#[derive(Debug, Serialize)]
pub struct ChallengeResponse {
    pub salt: u64,
}

/// 存储驱动信息
#[derive(Debug, Serialize)]
pub struct StorageDriverInfo {
    pub value: String,
    pub label: String,
}

/// 存储模板响应
#[derive(Debug, Serialize)]
pub struct StorageTemplateResponse {
    pub driver: String,
    pub template: serde_json::Value,
}

/// 添加存储请求
#[derive(Debug, Deserialize)]
pub struct AddStorageRequest {
    pub prefix: String,
    pub driver: crate::storage::all::AllDriverConfigMeta,
    pub public: Option<bool>,
}

/// 添加存储响应
#[derive(Debug, Serialize)]
pub struct AddStorageResponse {
    pub prefix: String,
    pub driver: String,
    pub message: String,
}

/// 公开节点操作请求（带 Challenge 验证）
#[derive(Debug, Deserialize)]
pub struct PublicFsRequest {
    pub path: Option<String>,
    /// 页码（从 0 开始），与 cursor 二选一
    pub page: Option<u32>,
    /// 游标（页码，从 0 开始），与 page 二选一
    pub cursor: Option<usize>,
    /// 每页数量，默认 20
    pub per_page: Option<u32>,
    #[serde(deserialize_with = "deserialize_u64_from_str_or_num")]
    pub salt: u64,
    #[serde(deserialize_with = "deserialize_u64_from_str_or_num")]
    pub timestamp: u64,
    pub nonce: String,
    pub claim: String,
}
