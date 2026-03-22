//! 中国移动云盘错误类型

use thiserror::Error;

use crate::error::{NetworkError, RlistError, SerializationError, StorageError};

#[derive(Error, Debug)]
pub enum McloudError {
    #[error("认证失败：{0}")]
    AuthError(String),

    #[error("Token 已过期")]
    TokenExpired,

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
}

impl From<reqwest::Error> for McloudError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            McloudError::NetworkError("请求超时".to_string())
        } else if err.is_request() {
            McloudError::ApiError(format!("请求错误：{}", err))
        } else {
            McloudError::NetworkError(err.to_string())
        }
    }
}

impl From<serde_json::Error> for McloudError {
    fn from(err: serde_json::Error) -> Self {
        McloudError::ParseError(format!("JSON 解析失败：{}", err))
    }
}

impl From<StorageError> for McloudError {
    fn from(err: StorageError) -> Self {
        McloudError::ApiError(err.to_string())
    }
}

impl From<McloudError> for RlistError {
    fn from(err: McloudError) -> Self {
        match err {
            McloudError::AuthError(_e) => RlistError::Network(NetworkError::RequestFailed),
            McloudError::TokenExpired => RlistError::Network(NetworkError::RequestFailed),
            McloudError::ApiError(_e) => RlistError::Network(NetworkError::RequestFailed),
            McloudError::NetworkError(_e) => RlistError::Network(NetworkError::RequestFailed),
            McloudError::NotFound(_e) => RlistError::Storage(StorageError::NotFound),
            McloudError::DownloadError(_e) => RlistError::Network(NetworkError::RequestFailed),
            McloudError::ParseError(_e) => RlistError::Serialization(SerializationError::Parse),
        }
    }
}
