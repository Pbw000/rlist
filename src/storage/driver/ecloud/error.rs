//! 天翼云盘错误类型

use thiserror::Error;

use crate::error::{NetworkError, RlistError, SerializationError, StorageError};

#[derive(Error, Debug)]
pub enum EcloudError {
    #[error("认证失败：{0}")]
    AuthError(String),

    #[error("Token 已过期：{0}")]
    TokenExpired(String),

    #[error("API 请求失败：{0}")]
    ApiError(String),

    #[error("网络错误：{0}")]
    NetworkError(String),

    #[error("文件不存在：{0}")]
    NotFound(String),

    #[error("下载失败：{0}")]
    DownloadError(String),

    #[error("解析错误：{0}")]
    ParseError(String),

    #[error("上传失败：{0}")]
    UploadError(String),

    #[error("验证码错误：{0}")]
    CaptchaError(String),
}

impl From<String> for EcloudError {
    fn from(err: String) -> Self {
        EcloudError::ApiError(err)
    }
}

impl From<reqwest::Error> for EcloudError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            EcloudError::NetworkError("请求超时".to_string())
        } else if err.is_request() {
            EcloudError::ApiError(format!("请求错误：{}", err))
        } else {
            EcloudError::NetworkError(err.to_string())
        }
    }
}

impl From<serde_json::Error> for EcloudError {
    fn from(err: serde_json::Error) -> Self {
        EcloudError::ParseError(format!("JSON 解析失败：{}", err))
    }
}

impl From<StorageError> for EcloudError {
    fn from(err: StorageError) -> Self {
        EcloudError::ApiError(err.to_string())
    }
}

impl From<EcloudError> for RlistError {
    fn from(err: EcloudError) -> Self {
        match err {
            EcloudError::AuthError(e) => RlistError::Network(NetworkError::RequestFailed(e)),
            EcloudError::TokenExpired(e) => RlistError::Network(NetworkError::RequestFailed(e)),
            EcloudError::ApiError(e) => RlistError::Network(NetworkError::RequestFailed(e)),
            EcloudError::NetworkError(e) => RlistError::Network(NetworkError::RequestFailed(e)),
            EcloudError::NotFound(e) => RlistError::Storage(StorageError::NotFound(e)),
            EcloudError::DownloadError(e) => RlistError::Network(NetworkError::RequestFailed(e)),
            EcloudError::ParseError(e) => RlistError::Serialization(SerializationError::Parse(e)),
            EcloudError::UploadError(e) => RlistError::Storage(StorageError::OperationFailed(e)),
            EcloudError::CaptchaError(e) => RlistError::Network(NetworkError::RequestFailed(e)),
        }
    }
}
