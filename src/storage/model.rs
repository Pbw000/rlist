//! 存储驱动模型和统一接口定义
//!
//! 提供统一的存储抽象层，包括文件元数据和存储操作 trait

use std::error::Error;
use std::future::Future;

use tokio::io::{AsyncRead, AsyncSeek};
use tokio::sync::RwLock;

// 重新导出 meta 模块的类型
pub use crate::meta::{FileType, Meta};

/// 文件元数据（统一抽象类型）
pub type FileMeta = Meta;

/// 文件列表响应
#[derive(Debug, Clone)]
pub struct FileList {
    /// 文件列表
    pub items: Vec<FileMeta>,
    /// 总数
    pub total: u64,
    /// 下一页游标（如果有）
    pub next_cursor: Option<String>,
}

impl FileList {
    pub fn new(items: Vec<FileMeta>, total: u64) -> Self {
        Self {
            items,
            total,
            next_cursor: None,
        }
    }

    pub fn with_cursor(items: Vec<FileMeta>, total: u64, next_cursor: Option<String>) -> Self {
        Self {
            items,
            total,
            next_cursor,
        }
    }
}

/// 文件内容读取器 trait
pub trait FileContent: AsyncRead + AsyncSeek + Send + Sync {
    fn size(&self) -> Option<u64>;
}

/// 存储操作统一 trait
pub trait Storage: Send + Sync {
    type Error: Send + Sync + Error + 'static;

    fn name(&self) -> &str;
    fn driver_name(&self) -> &str;
    fn is_readonly(&self) -> bool {
        false
    }

    fn list_files(
        &self,
        path: &str,
        page_size: u32,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<FileList, Self::Error>> + Send;

    fn get_meta(&self, path: &str) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send;

    fn get_download_url(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<String, Self::Error>> + Send;

    fn download_file(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<Box<dyn FileContent>, Self::Error>> + Send;

    fn create_folder(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send;

    fn delete(&self, path: &str) -> impl Future<Output = Result<(), Self::Error>> + Send;

    fn rename(
        &self,
        old_path: &str,
        new_name: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send;

    fn copy(
        &self,
        source_path: &str,
        dest_path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send;

    fn move_(
        &self,
        source_path: &str,
        dest_path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send;

    fn upload_file(
        &self,
        path: &str,
        content: Vec<u8>,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send;

    fn from_auth_data(json: &str) -> Result<Self, Self::Error>
    where
        Self: Sized;

    fn auth_template() -> String
    where
        Self: Sized;
}

/// 存储驱动 trait（向后兼容）
pub trait StorageDriver: Send + Sync {
    type StorageError: Send + Sync + Error + 'static;
    fn name(&self) -> &str;
    fn handle_path(&self, path: &str) -> Result<String, Self::StorageError>;

    fn from_auth_data(json: &str) -> Result<Self, Self::StorageError>
    where
        Self: Sized;

    fn auth_template() -> String
    where
        Self: Sized;

    fn is_readonly(&self) -> bool {
        false
    }
}

/// 存储注册表
pub struct StorageRegistry<T: StorageDriver> {
    drivers: Vec<RwLock<T>>,
}

impl<T: StorageDriver> StorageRegistry<T> {
    pub fn new() -> Self {
        todo!()
    }
}

impl<T: StorageDriver> Default for StorageRegistry<T> {
    fn default() -> Self {
        todo!()
    }
}
