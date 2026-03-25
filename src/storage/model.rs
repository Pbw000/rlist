//! 存储驱动模型和统一接口定义
//!
//! 提供统一的存储抽象层，包括文件元数据和存储操作 trait

use std::error::Error;
use std::future::Future;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncSeek};

use crate::error::RlistError;
use crate::storage::file_meta::DownloadableMeta;

// 重新导出 meta 模块的类型
pub use super::file_meta::Meta;

/// 文件元数据（统一抽象类型）
pub type FileMeta = Meta;

/// 文件列表响应
#[derive(Debug, Clone, Deserialize, Serialize)]
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
pub trait FileContent: AsyncRead + AsyncSeek + Send + Sync + Unpin {
    fn size(&self) -> Option<u64>;
}

/// 上传模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UploadMode {
    /// 中继模式：文件通过服务器中转上传
    Relay,
    /// Direct 模式：直接返回上传链接，客户端直接上传到存储端
    Direct,
}

/// 上传信息请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadInfoParams {
    /// 文件路径
    pub path: String,
    /// 文件大小
    pub size: u64,
    ///SHA-256
    pub hash: String,
}

/// 上传信息（用于 Direct 模式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadInfo {
    /// 上传 URL
    pub upload_url: String,
    /// 上传方法 (POST/PUT 等)
    pub method: String,
    /// 上传表单字段（如果需要）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form_fields: Option<std::collections::HashMap<String, String>>,
    /// 上传请求头（如果需要）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// 上传完成回调 URL（Direct 模式下，前端上传完成后需调用此接口通知后端）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complete_url: Option<String>,
}

pub trait Storage: Send + Sync {
    type Error: Send + Sync + Error + 'static + Into<RlistError> + From<String>;
    type End2EndCopyMeta: Send;
    type End2EndMoveMeta: Send;

    /// 存储名称（人类可读）
    fn name(&self) -> &str;
    fn hash(&self) -> u64;

    /// 驱动名称（标识符）
    fn driver_name(&self) -> &str {
        self.name()
    }

    fn build_cache(&self, _path: &str) -> impl Future<Output = Result<(), Self::Error>> + Send {
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
    fn copy_end_to_end(
        &self,
        source_meta: Self::End2EndCopyMeta,
        dest_path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send;

    fn gen_copy_meta(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<Self::End2EndCopyMeta, Self::Error>> + Send;
    /// 移动
    fn move_end_to_end(
        &self,
        source_meta: Self::End2EndMoveMeta,
        dest_path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send;

    fn gen_move_meta(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<Self::End2EndMoveMeta, Self::Error>> + Send;

    fn get_upload_info(
        &self,
        params: UploadInfoParams,
    ) -> impl Future<Output = Result<UploadInfo, Self::Error>> + Send
    where
        Self: Sized;

    /// 完成上传（Direct 模式）
    /// 前端上传完成后调用此方法通知后端完成上传流程
    /// 默认返回 Ok(None) 表示不需要完成步骤
    fn complete_upload(
        &self,
        _path: &str,
        _upload_id: &str,
        _file_id: &str,
        _content_hash: &str,
    ) -> impl Future<Output = Result<Option<FileMeta>, Self::Error>> + Send
    where
        Self: Sized,
    {
        async move { Ok(None) }
    }

    /// 上传文件（中继模式）
    /// 文件内容通过服务器中转上传到存储端
    /// 使用流式上传，支持大文件
    fn upload_file<R: tokio::io::AsyncRead + Send + Unpin + 'static>(
        &self,
        path: &str,
        content: R,
        param: UploadInfoParams,
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
