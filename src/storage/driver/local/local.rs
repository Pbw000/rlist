use crate::error::StorageError;
use crate::storage::file_meta::DownloadableMeta;
use crate::storage::model::{FileContent, FileList, FileMeta, Storage};
use ring::digest::{Context, SHA256};
use std::future::Future;
use std::io::SeekFrom;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek};

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
        // dbg!(&full_path);

        let canonical_root = self
            .root
            .canonicalize()
            .map_err(|e| StorageError::NotFound(e.to_string()))?;
        // dbg!(&canonical_root);

        let canonical_path = full_path
            .canonicalize()
            .unwrap_or_else(|_| full_path.clone());
        // dbg!(&canonical_path);

        if !canonical_path.starts_with(&canonical_root) {
            return Err(StorageError::NotFound("路径遍历被阻止".to_string()));
        }

        Ok(canonical_path)
    }

    fn meta_from_path(&self, path: &PathBuf) -> Result<FileMeta, StorageError> {
        use chrono::DateTime;

        let metadata =
            std::fs::metadata(path).map_err(|e| StorageError::NotFound(e.to_string()))?;
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

            let entries =
                std::fs::read_dir(&dir_path).map_err(|e| StorageError::NotFound(e.to_string()))?;

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
            let metadata = std::fs::metadata(&normalized)
                .map_err(|e| StorageError::NotFound(e.to_string()))?;

            let mut file = File::open(&normalized)
                .await
                .map_err(|e| StorageError::NotFound(e.to_string()))?;
            let mut check_sum = Context::new(&SHA256);
            let mut buffer = [0; 1024];

            loop {
                let bytes_read = file
                    .read(&mut buffer)
                    .await
                    .map_err(|e| StorageError::OperationFailed(e.to_string()))?;
                if bytes_read == 0 {
                    break;
                }
                check_sum.update(&buffer[..bytes_read]);
            }

            // 生成 API 下载端点 URL
            let download_url = format!("/api/fs/download?path={}", path);

            Ok(DownloadableMeta {
                download_url,
                size: metadata.len(),
                hash: hex::encode(check_sum.finish()),
            })
        }
    }

    fn download_file(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<Box<dyn FileContent>, Self::Error>> + Send {
        async move {
            let normalized = self.normalize_path(path)?;
            let metadata = std::fs::metadata(&normalized)
                .map_err(|e| StorageError::NotFound(e.to_string()))?;

            let file = File::open(&normalized)
                .await
                .map_err(|e| StorageError::NotFound(e.to_string()))?;
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
            std::fs::create_dir_all(&normalized)
                .map_err(|e| StorageError::OperationFailed(e.to_string()))?;
            self.meta_from_path(&normalized)
        }
    }

    fn delete(&self, path: &str) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            let normalized = self.normalize_path(path)?;
            if normalized.is_dir() {
                std::fs::remove_dir_all(&normalized)
                    .map_err(|e| StorageError::OperationFailed(e.to_string()))?;
            } else {
                std::fs::remove_file(&normalized)
                    .map_err(|e| StorageError::OperationFailed(e.to_string()))?;
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
            let parent = normalized
                .parent()
                .ok_or_else(|| StorageError::OperationFailed("无法获取父目录".to_string()))?;
            let new_path = parent.join(new_name);

            std::fs::rename(&normalized, &new_path)
                .map_err(|e| StorageError::OperationFailed(e.to_string()))?;
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
                copy_dir_recursive(&source, &dest)
                    .map_err(|e| StorageError::OperationFailed(e.to_string()))?;
            } else {
                std::fs::copy(&source, &dest)
                    .map_err(|e| StorageError::OperationFailed(e.to_string()))?;
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

            std::fs::rename(&source, &dest)
                .map_err(|e| StorageError::OperationFailed(e.to_string()))?;
            self.meta_from_path(&dest)
        }
    }

    fn upload_file<R: tokio::io::AsyncRead + Send + Unpin + 'static>(
        &self,
        path: &str,
        mut content: R,
        _param: crate::storage::model::UploadInfoParams,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send {
        async move {
            let normalized = self.normalize_path(path)?;

            if let Some(parent) = normalized.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| StorageError::OperationFailed(e.to_string()))?;
            }

            // 使用 tokio 异步写入文件
            let mut file = tokio::fs::File::create(&normalized)
                .await
                .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

            // 复制内容到文件
            tokio::io::copy(&mut content, &mut file)
                .await
                .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

            self.meta_from_path(&normalized)
        }
    }

    fn upload_mode(&self) -> crate::storage::model::UploadMode {
        // 本地存储支持 Direct 模式，直接返回文件路径
        crate::storage::model::UploadMode::Direct
    }

    fn get_upload_info(
        &self,
        params: crate::storage::model::UploadInfoParams,
    ) -> impl Future<Output = Result<crate::storage::model::UploadInfo, Self::Error>> + Send {
        async move {
            let normalized = self.normalize_path(&params.path)?;
            Ok(crate::storage::model::UploadInfo {
                upload_url: format!("file://{}", normalized.to_string_lossy()),
                method: "PUT".to_string(),
                form_fields: None,
                headers: None,
                complete_url: None, // 本地存储无需 complete
            })
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
