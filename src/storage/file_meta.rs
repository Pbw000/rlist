//! 文件和目录元数据定义模块
//!
//! 提供统一的文件和目录抽象，用于不同存储后端的数据表示

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::storage::model::Hash;

/// 文件/目录元数据 - 最小实现
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Meta {
    /// 文件元数据
    File {
        /// 文件名
        name: String,
        /// 文件大小（字节）
        size: u64,
        /// 修改时间
        modified_at: Option<DateTime<Utc>>,
    },
    /// 目录元数据
    Directory {
        /// 目录名
        name: String,
        /// 修改时间
        modified_at: Option<DateTime<Utc>>,
    },
}

impl Meta {
    /// 创建文件元数据
    pub fn file(name: impl Into<String>, size: u64) -> Self {
        Self::File {
            name: name.into(),
            size,
            modified_at: None,
        }
    }

    /// 创建目录元数据
    pub fn directory(name: impl Into<String>) -> Self {
        Self::Directory {
            name: name.into(),
            modified_at: None,
        }
    }

    /// 获取名称
    pub fn name(&self) -> &str {
        match self {
            Meta::File { name, .. } | Meta::Directory { name, .. } => name,
        }
    }

    /// 判断是否为文件
    pub fn is_file(&self) -> bool {
        matches!(self, Meta::File { .. })
    }

    /// 判断是否为目录
    pub fn is_dir(&self) -> bool {
        matches!(self, Meta::Directory { .. })
    }

    /// 获取人类可读的大小
    pub fn human_size(&self) -> String {
        let s = match self {
            Meta::File { size, .. } => *size,
            Meta::Directory { .. } => 0,
        };
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        const TB: u64 = GB * 1024;

        match s {
            s if s < KB => format!("{} B", s),
            s if s < MB => format!("{:.2} KB", s as f64 / KB as f64),
            s if s < GB => format!("{:.2} MB", s as f64 / MB as f64),
            s if s < TB => format!("{:.2} GB", s as f64 / GB as f64),
            s => format!("{:.2} TB", s as f64 / TB as f64),
        }
    }
}
/// 可下载文件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadableMeta {
    pub download_url: String,
    pub size: u64,
    pub hash: Hash,
}
