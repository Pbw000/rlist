//! 存储驱动模型和统一接口定义
//!
//! 提供统一的存储抽象层，包括文件元数据和存储操作 trait

use std::error::Error;
use std::future::Future;

use tokio::io::{AsyncRead, AsyncSeek};

use crate::error::RlistError;
use crate::storage::file_meta::DownloadableMeta;

// 重新导出 meta 模块的类型
pub use super::file_meta::Meta;

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
    type Error: Send + Sync + Error + 'static + Into<RlistError>;

    /// 存储名称（人类可读）
    fn name(&self) -> &str;

    /// 驱动名称（标识符）
    fn driver_name(&self) -> &str {
        self.name()
    }

    /// 是否只读
    fn is_readonly(&self) -> bool {
        false
    }

    /// 构建缓存（可选实现）
    fn build_cache(&self) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }

    /// 处理路径，返回元数据
    fn handle_path(&self, path: &str)
    -> impl Future<Output = Result<FileMeta, Self::Error>> + Send;

    /// 列出文件
    fn list_files(
        &self,
        path: &str,
        page_size: u32,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<FileList, Self::Error>> + Send;

    /// 获取元数据
    fn get_meta(&self, path: &str) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send;

    /// 获取可下载元数据
    fn get_download_meta_by_path(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<DownloadableMeta, Self::Error>> + Send;

    /// 下载文件
    fn download_file(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<Box<dyn FileContent>, Self::Error>> + Send;

    /// 创建文件夹
    fn create_folder(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send;

    /// 删除
    fn delete(&self, path: &str) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// 重命名
    fn rename(
        &self,
        old_path: &str,
        new_name: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send;

    /// 复制
    fn copy(
        &self,
        source_path: &str,
        dest_path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send;

    /// 移动
    fn move_(
        &self,
        source_path: &str,
        dest_path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send;

    /// 上传文件
    fn upload_file(
        &self,
        path: &str,
        content: Vec<u8>,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send;

    /// 从认证数据创建实例
    fn from_auth_data(json: &str) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// 认证模板
    fn auth_template(&self) -> String
    where
        Self: Sized;
}
