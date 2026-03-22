//! 文件和目录元数据定义模块
//!
//! 提供统一的文件和目录抽象，用于不同存储后端的数据表示

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 文件类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    /// 普通文件
    File,
    /// 目录
    Directory,
}

/// 文件/目录元数据 - 最小实现
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    /// 文件名
    pub name: String,
    /// 完整路径
    pub path: PathBuf,
    /// 文件类型
    pub file_type: FileType,
    /// 文件大小（字节）
    pub size: u64,
    /// 修改时间
    pub modified_at: Option<DateTime<Utc>>,
}

impl Meta {
    /// 创建文件元数据
    pub fn file(name: impl Into<String>, path: PathBuf, size: u64) -> Self {
        Self {
            name: name.into(),
            path,
            file_type: FileType::File,
            size,
            modified_at: None,
        }
    }

    /// 创建目录元数据
    pub fn directory(name: impl Into<String>, path: PathBuf) -> Self {
        Self {
            name: name.into(),
            path,
            file_type: FileType::Directory,
            size: 0,
            modified_at: None,
        }
    }

    /// 判断是否为文件
    pub fn is_file(&self) -> bool {
        self.file_type == FileType::File
    }

    /// 判断是否为目录
    pub fn is_dir(&self) -> bool {
        self.file_type == FileType::Directory
    }

    /// 获取人类可读的大小
    pub fn human_size(&self) -> String {
        let s = self.size;
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
