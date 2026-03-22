//! 错误类型定义模块
//!
//! 提供统一的错误处理机制，覆盖存储操作、网络请求、序列化等场景

use thiserror::Error;

/// 统一错误类型
#[derive(Error, Debug)]
pub enum RlistError {
    /// 存储后端错误
    #[error("存储错误：{0}")]
    Storage(#[from] StorageError),

    /// 网络请求错误
    #[error("网络错误：{0}")]
    Network(#[from] NetworkError),

    /// 序列化/反序列化错误
    #[error("序列化错误：{0}")]
    Serialization(#[from] SerializationError),

    /// 加密/解密错误
    #[error("加密错误：{0}")]
    Crypto(#[from] CryptoError),

    /// 路径解析错误
    #[error("路径错误：{0}")]
    Path(#[from] PathError),

    /// IO 错误
    #[error("IO 错误：{0}")]
    Io(#[from] std::io::Error),
}

/// 存储后端相关错误
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("存储未找到")]
    NotFound,

    #[error("存储已存在")]
    AlreadyExists,

    #[error("权限拒绝")]
    PermissionDenied,

    #[error("存储配置无效")]
    InvalidConfig,

    #[error("存储操作失败")]
    OperationFailed,

    #[error("不支持的操作")]
    Unsupported,
}
/// 网络相关错误
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("请求失败")]
    RequestFailed,

    #[error("连接超时")]
    Timeout,

    #[error("无效 URL")]
    InvalidUrl,

    #[error("HTTP 错误")]
    Http,

    #[error("TLS 错误")]
    TlsError,
}

/// 序列化/反序列化错误
#[derive(Error, Debug)]
pub enum SerializationError {
    #[error("解析错误")]
    Parse,
    #[error("JSON 错误")]
    Json(#[from] serde_json::Error),

    #[error("Postcard 错误")]
    Postcard,

    #[error("无效数据格式")]
    InvalidData,
}

/// 加密相关错误
#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("密钥派生失败")]
    KeyDerivation,

    #[error("加密失败")]
    Encryption,

    #[error("解密失败")]
    Decryption,

    #[error("签名验证失败")]
    SignatureInvalid,

    #[error("哈希错误")]
    Hash,
}

/// 路径相关错误
#[derive(Error, Debug)]
pub enum PathError {
    #[error("无效路径")]
    InvalidPath,

    #[error("路径遍历攻击被阻止")]
    TraversalAttempt,

    #[error("路径解析失败")]
    ParseFailed,

    #[error("根路径错误")]
    RootPath,
}

impl From<reqwest::Error> for RlistError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            RlistError::Network(NetworkError::Timeout)
        } else if err.is_request() {
            RlistError::Network(NetworkError::RequestFailed)
        } else {
            RlistError::Network(NetworkError::RequestFailed)
        }
    }
}

impl From<postcard::Error> for RlistError {
    fn from(_: postcard::Error) -> Self {
        RlistError::Serialization(SerializationError::Postcard)
    }
}

/// 结果类型别名
pub type RlistResult<T> = std::result::Result<T, RlistError>;
