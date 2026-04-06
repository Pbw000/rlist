//! 联通云盘错误类型

use thiserror::Error;

#[derive(Debug, Error)]
pub enum WopanError {
    #[error("API 错误：{0}")]
    ApiError(String),

    #[error("认证失败：{0}")]
    AuthError(String),

    #[error("令牌过期：{0}")]
    TokenExpired(String),

    #[error("未找到：{0}")]
    NotFound(String),

    #[error("网络错误：{0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("JSON 解析错误：{0}")]
    ParseError(String),

    #[error("上传错误：{0}")]
    UploadError(String),

    #[error("下载错误：{0}")]
    DownloadError(String),

    #[error("加密错误：{0}")]
    CryptoError(String),
}

impl From<String> for WopanError {
    fn from(s: String) -> Self {
        WopanError::ApiError(s)
    }
}

impl From<&str> for WopanError {
    fn from(s: &str) -> Self {
        WopanError::ApiError(s.to_string())
    }
}

impl From<WopanError> for crate::error::RlistError {
    fn from(err: WopanError) -> Self {
        crate::error::RlistError::Storage(crate::error::StorageError::Custom(err.to_string()))
    }
}
