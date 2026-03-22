//! 本地存储驱动实现

use crate::error::StorageError;
use crate::storage::model::{FileContent, FileList, FileMeta, StorageDriver};
use std::future::Future;
use std::io::SeekFrom;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncSeek};

/// 本地存储
pub struct LocalStorage {
    root: PathBuf,
}

impl LocalStorage {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn normalize_path(&self, path: &str) -> Result<PathBuf, StorageError> {
        let path = path.trim_start_matches('/');
        let full_path = self.root.join(path);

        let canonical_root = self
            .root
            .canonicalize()
            .map_err(|_| StorageError::PermissionDenied)?;
        let canonical_path = full_path
            .canonicalize()
            .unwrap_or_else(|_| full_path.clone());

        if !canonical_path.starts_with(&canonical_root) {
            return Err(StorageError::PermissionDenied);
        }

        Ok(canonical_path)
    }

    fn meta_from_path(&self, path: &PathBuf) -> Result<FileMeta, StorageError> {
        use crate::meta::{FileType, Meta};
        use chrono::DateTime;

        let metadata = std::fs::metadata(path).map_err(|_| StorageError::NotFound)?;

        let file_type = if metadata.is_dir() {
            FileType::Directory
        } else {
            FileType::File
        };

        let mut meta = if file_type == FileType::Directory {
            Meta::directory(
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown"),
                path.clone(),
            )
        } else {
            Meta::file(
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown"),
                path.clone(),
                metadata.len(),
            )
        };

        if let Ok(modified) = metadata.modified() {
            meta.modified_at = Some(DateTime::from(modified));
        }

        Ok(meta)
    }
}

impl StorageDriver for LocalStorage {
    type StorageError = crate::error::StorageError;

    fn name(&self) -> &str {
        "local"
    }

    fn handle_path(&self, path: &str) -> Result<String, Self::StorageError> {
        let normalized = self.normalize_path(path)?;
        Ok(normalized.to_string_lossy().to_string())
    }

    fn from_auth_data(_json: &str) -> Result<Self, Self::StorageError>
    where
        Self: Sized,
    {
        Ok(Self::new("."))
    }

    fn auth_template() -> String {
        r#"{"type": "none"}"#.to_string()
    }
}

/// 本地文件读取器
pub struct LocalFileReader {
    file: File,
    size: Option<u64>,
}

impl LocalFileReader {
    pub fn new(file: File, size: Option<u64>) -> Self {
        Self { file, size }
    }
}

impl AsyncRead for LocalFileReader {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        AsyncRead::poll_read(std::pin::Pin::new(&mut self.file), cx, buf)
    }
}

impl AsyncSeek for LocalFileReader {
    fn start_seek(mut self: std::pin::Pin<&mut Self>, position: SeekFrom) -> std::io::Result<()> {
        AsyncSeek::start_seek(std::pin::Pin::new(&mut self.file), position)
    }

    fn poll_complete(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<u64>> {
        AsyncSeek::poll_complete(std::pin::Pin::new(&mut self.file), cx)
    }
}

impl FileContent for LocalFileReader {
    fn size(&self) -> Option<u64> {
        self.size
    }
}

// ============== 统一的 Storage trait 实现 ==============

impl crate::storage::model::Storage for LocalStorage {
    type Error = StorageError;

    fn name(&self) -> &str {
        "本地存储"
    }

    fn driver_name(&self) -> &str {
        "local"
    }

    fn is_readonly(&self) -> bool {
        false
    }

    fn list_files(
        &self,
        path: &str,
        _page_size: u32,
        _cursor: Option<String>,
    ) -> impl Future<Output = Result<FileList, Self::Error>> + Send {
        async move {
            let dir_path = self.normalize_path(path)?;

            let entries = std::fs::read_dir(&dir_path).map_err(|_| StorageError::NotFound)?;

            let mut items = Vec::new();
            let mut total = 0u64;

            for entry in entries.flatten() {
                let entry_path = entry.path();
                match self.meta_from_path(&entry_path) {
                    Ok(meta) => {
                        total += 1;
                        items.push(meta);
                    }
                    Err(_) => continue,
                }
            }

            Ok(FileList::new(items, total))
        }
    }

    fn get_meta(&self, path: &str) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send {
        async move {
            let normalized = self.normalize_path(path)?;
            self.meta_from_path(&normalized)
        }
    }

    fn get_download_url(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<String, Self::Error>> + Send {
        async move {
            let normalized = self.normalize_path(path)?;
            Ok(normalized.to_string_lossy().to_string())
        }
    }

    fn download_file(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<Box<dyn FileContent>, Self::Error>> + Send {
        async move {
            let normalized = self.normalize_path(path)?;
            let metadata = std::fs::metadata(&normalized).map_err(|_| StorageError::NotFound)?;

            let file = File::open(&normalized)
                .await
                .map_err(|_| StorageError::NotFound)?;
            let reader: Box<dyn FileContent> =
                Box::new(LocalFileReader::new(file, Some(metadata.len())));
            Ok(reader)
        }
    }

    fn create_folder(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send {
        async move {
            let normalized = self.normalize_path(path)?;
            std::fs::create_dir_all(&normalized).map_err(|_| StorageError::OperationFailed)?;
            self.meta_from_path(&normalized)
        }
    }

    fn delete(&self, path: &str) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            let normalized = self.normalize_path(path)?;
            if normalized.is_dir() {
                std::fs::remove_dir_all(&normalized).map_err(|_| StorageError::OperationFailed)?;
            } else {
                std::fs::remove_file(&normalized).map_err(|_| StorageError::OperationFailed)?;
            }
            Ok(())
        }
    }

    fn rename(
        &self,
        old_path: &str,
        new_name: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send {
        async move {
            let normalized = self.normalize_path(old_path)?;
            let parent = normalized.parent().ok_or(StorageError::OperationFailed)?;
            let new_path = parent.join(new_name);

            std::fs::rename(&normalized, &new_path).map_err(|_| StorageError::OperationFailed)?;
            self.meta_from_path(&new_path)
        }
    }

    fn copy(
        &self,
        source_path: &str,
        dest_path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send {
        async move {
            let source = self.normalize_path(source_path)?;
            let dest = self.normalize_path(dest_path)?;

            if source.is_dir() {
                copy_dir_recursive(&source, &dest).map_err(|_| StorageError::OperationFailed)?;
            } else {
                std::fs::copy(&source, &dest).map_err(|_| StorageError::OperationFailed)?;
            }

            self.meta_from_path(&dest)
        }
    }

    fn move_(
        &self,
        source_path: &str,
        dest_path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send {
        async move {
            let source = self.normalize_path(source_path)?;
            let dest = self.normalize_path(dest_path)?;

            std::fs::rename(&source, &dest).map_err(|_| StorageError::OperationFailed)?;
            self.meta_from_path(&dest)
        }
    }

    fn upload_file(
        &self,
        path: &str,
        content: Vec<u8>,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send {
        async move {
            let normalized = self.normalize_path(path)?;

            if let Some(parent) = normalized.parent() {
                std::fs::create_dir_all(parent).map_err(|_| StorageError::OperationFailed)?;
            }

            std::fs::write(&normalized, &content).map_err(|_| StorageError::OperationFailed)?;
            self.meta_from_path(&normalized)
        }
    }

    fn from_auth_data(json: &str) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        <LocalStorage as StorageDriver>::from_auth_data(json)
    }

    fn auth_template() -> String
    where
        Self: Sized,
    {
        <LocalStorage as StorageDriver>::auth_template()
    }
}

/// 递归复制目录
fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;

        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }

    Ok(())
}
