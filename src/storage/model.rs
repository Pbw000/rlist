//! 存储驱动模型和统一接口定义
//!
//! 提供统一的存储抽象层，包括文件元数据和存储操作 trait

use std::error::Error;
use std::fmt::Debug;
use std::future::Future;

use serde::de::DeserializeOwned;
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
    fn hash(&self) -> &str;
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
    type End2EndCopyMeta: Send + Debug;
    type End2EndMoveMeta: Send + Debug;
    type ConfigMeta: Send + Serialize + DeserializeOwned + Default;
    fn name(&self) -> &str;
    fn hash(&self) -> u64;
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
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// 复制
    fn copy_end_to_end(
        &self,
        source_meta: Self::End2EndCopyMeta,
        dest_path: &str,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    fn gen_copy_meta(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<Self::End2EndCopyMeta, Self::Error>> + Send;
    /// 移动
    fn move_end_to_end(
        &self,
        source_meta: Self::End2EndMoveMeta,
        dest_path: &str,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

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
    fn upload_file<R: tokio::io::AsyncRead + Send + Unpin + 'static>(
        &self,
        path: &str,
        content: R,
        param: UploadInfoParams,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send;
    fn copy_relay(
        &self,
        source_path: &str,
        dest_path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send
    where
        Self: Sized,
    {
        async move {
            let source_meta = self.get_meta(source_path).await?;
            let source_size = match &source_meta {
                Meta::File { size, .. } => *size,
                Meta::Directory { .. } => {
                    return Err(Self::Error::from("Cannot copy directory".to_string()));
                }
            };

            let content = self.download_file(source_path).await?;

            // 获取 hash
            let hash = content.hash().to_string();

            // 构建上传参数
            let upload_param = UploadInfoParams {
                path: dest_path.to_string(),
                size: source_size,
                hash,
            };

            self.upload_file(dest_path, content, upload_param).await
        }
    }

    fn move_file(
        &self,
        source_path: &str,
        dest_path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send
    where
        Self: Sized,
    {
        async move {
            let meta = self.copy_relay(source_path, dest_path).await?;
            self.delete(source_path).await?;
            Ok(meta)
        }
    }

    /// 从认证数据创建实例
    fn from_auth_data(data: Self::ConfigMeta) -> Result<Self, Self::Error>
    where
        Self: Sized;

    fn auth_template() -> Self::ConfigMeta
    where
        Self: Sized,
    {
        Self::ConfigMeta::default()
    }
    fn to_auth_data(&self) -> Self::ConfigMeta;
}
