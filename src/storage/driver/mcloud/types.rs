//! 中国移动云盘类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 文件类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum McloudFileType {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "folder")]
    Folder,
}

/// 文件元数据 - 最小实现
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McloudFileMeta {
    #[serde(rename = "fileId")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "type")]
    pub file_type: McloudFileType,
    #[serde(rename = "size")]
    pub size: Option<u64>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
}

impl McloudFileMeta {
    /// 转换为统一的 Meta 类型
    pub fn to_meta(&self) -> crate::meta::Meta {
        use crate::meta::{FileType, Meta};

        let file_type = match self.file_type {
            McloudFileType::File => FileType::File,
            McloudFileType::Folder => FileType::Directory,
        };

        let mut meta = if file_type == FileType::Directory {
            Meta::directory(&self.name, std::path::PathBuf::from(&self.id))
        } else {
            Meta::file(
                &self.name,
                std::path::PathBuf::from(&self.id),
                self.size.unwrap_or(0),
            )
        };

        // 解析修改时间
        if let Some(updated) = &self.updated_at {
            meta.modified_at = DateTime::parse_from_rfc3339(updated)
                .map(|d| d.with_timezone(&Utc))
                .ok();
        }

        meta
    }
}

/// 文件列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListResponse {
    pub items: Vec<McloudFileMeta>,
    pub total: Option<u32>,
    pub nextPageCursor: Option<String>,
    pub hasMore: Option<bool>,
}

impl FileListResponse {
    pub fn total(&self) -> u32 {
        self.total.unwrap_or_else(|| self.items.len() as u32)
    }

    pub fn files(&self) -> &Vec<McloudFileMeta> {
        &self.items
    }

    pub fn next_cursor(&self) -> Option<String> {
        self.nextPageCursor.clone()
    }

    pub fn to_file_list(&self) -> crate::storage::model::FileList {
        let items = self.items.iter().map(|f| f.to_meta()).collect();
        crate::storage::model::FileList::with_cursor(items, self.total() as u64, self.next_cursor())
    }
}

/// API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub code: Option<String>,
    pub message: Option<String>,
    pub data: Option<T>,
    pub success: Option<bool>,
}

impl<T> ApiResponse<T> {
    pub fn into_result(self) -> Result<T, String> {
        if self.success.unwrap_or(false) || self.code.as_deref() == Some("0") {
            self.data.ok_or_else(|| "No data in response".to_string())
        } else {
            Err(self
                .message
                .unwrap_or_else(|| self.code.unwrap_or_default()))
        }
    }
}
