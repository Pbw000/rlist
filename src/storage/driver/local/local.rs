//! 本地存储驱动实现

use crate::crypto::FileChecksum;
use crate::error::StorageError;
use crate::storage::file_meta::DownloadableMeta;
use crate::storage::model::{FileContent, FileList, FileMeta, Storage};
use std::future::Future;
use std::io::SeekFrom;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek};

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
        use chrono::DateTime;

        let metadata = std::fs::metadata(path).map_err(|_| StorageError::NotFound)?;
        let modified_at = metadata.modified().ok().map(DateTime::from);
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let meta = if metadata.is_dir() {
            FileMeta::Directory { name, modified_at }
        } else {
            FileMeta::File {
                name,
                size: metadata.len(),
                modified_at,
            }
        };

        Ok(meta)
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

// ============== Storage trait 实现 ==============

impl Storage for LocalStorage {
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

    async fn handle_path(&self, path: &str) -> Result<FileMeta, Self::Error> {
        let normalized = self.normalize_path(path)?;
        self.meta_from_path(&normalized)
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

    fn get_download_meta_by_path(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<DownloadableMeta, Self::Error>> + Send {
        async move {
            let normalized = self.normalize_path(path)?;
            let metadata = std::fs::metadata(&normalized).map_err(|_| StorageError::NotFound)?;

            let mut file = File::open(&normalized)
                .await
                .map_err(|_| StorageError::NotFound)?;
            let mut check_sum = FileChecksum::new();
            let mut buffer = [0; 1024];

            loop {
                let bytes_read = file
                    .read_exact(&mut buffer)
                    .await
                    .map_err(|_| StorageError::OperationFailed)?;
                if bytes_read == 0 {
                    break;
                }
                check_sum.update(&buffer[..bytes_read]);
            }

            Ok(DownloadableMeta {
                download_url: normalized.to_string_lossy().to_string(),
                size: metadata.len(),
                hash: check_sum.finish_hex(),
            })
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

    fn from_auth_data(_json: &str) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self::new("."))
    }

    fn auth_template(&self) -> String
    where
        Self: Sized,
    {
        r#"{"type": "none"}"#.to_string()
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
