//! 中国移动云盘 API 客户端
use std::sync::Arc;

use crate::storage::driver::mcloud::config::McloudConfig;
use crate::storage::driver::mcloud::error::McloudError;
use crate::storage::driver::mcloud::types::*;
use crate::storage::model::{FileContent, FileList, FileMeta, StorageDriver};
use crate::storage::radix_tree::{PathCache, PathCacheEntry};
use bytes::Bytes;
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::SeekFrom;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncSeek};

/// API 端点
const API_BASE: &str = "https://personal-kd-njs.yun.139.com";
const PERSONAL_NEW_BASE: &str = "/hcy";

/// 中国移动云盘存储
pub struct McloudStorage {
    config: McloudConfig,
    client: Arc<Client>,
    /// 路径缓存（使用 RadixTree 实现高效前缀匹配）
    path_cache: Arc<PathCache>,
}

impl McloudStorage {
    pub fn from_authorization(authorization: impl Into<String>) -> Result<Self, McloudError> {
        let authorization = authorization.into();
        let config = McloudConfig {
            authorization: authorization.clone(),
        };
        Ok(Self {
            client: Arc::new(Client::new()),
            config,
            path_cache: Arc::new(PathCache::new()),
        })
    }
}

impl StorageDriver for McloudStorage {
    type StorageError = McloudError;

    fn name(&self) -> &str {
        "mcloud"
    }

    fn handle_path(&self, path: &str) -> Result<String, Self::StorageError> {
        Ok(path.to_string())
    }

    fn from_auth_data(json: &str) -> Result<Self, Self::StorageError>
    where
        Self: Sized,
    {
        #[derive(serde::Deserialize)]
        struct AuthJson {
            authorization: String,
        }

        let auth_json: AuthJson = serde_json::from_str(json)
            .map_err(|_e| McloudError::ParseError("认证数据解析失败".to_string()))?;

        Ok(Self::from_authorization(auth_json.authorization)?)
    }

    fn auth_template() -> String {
        r#"{"type": "token", "fields": ["authorization"]}"#.to_string()
    }
}

impl McloudStorage {
    /// 创建新的客户端
    pub fn new(config: McloudConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .unwrap_or_default();

        Self {
            config,
            client: Arc::new(client),
            path_cache: Arc::new(PathCache::new()),
        }
    }

    pub fn with_client_arc(config: McloudConfig, client: Arc<Client>) -> Self {
        Self {
            config,
            client,
            path_cache: Arc::new(PathCache::new()),
        }
    }

    /// 构建请求
    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = if path.starts_with("http") {
            path.to_string()
        } else {
            format!("{}{}{}", API_BASE, PERSONAL_NEW_BASE, path)
        };

        let mut req = self.client.request(method, &url);

        // 添加请求头
        req = req
            .header("Origin", "https://yun.139.com")
            .header("Referer", "https://yun.139.com/")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36 Edg/146.0.0.0")
            .header("Authorization", format!("Basic {}", self.config.authorization))
            .header("x-yun-client-info", "||9|7.17.3|edge||6bce58bddf6e7f6dd7e961b4e740d82c||windows 10||zh-CN|||ZWRnZQ==||")
            .header("x-yun-module-type", "100")
            .header("x-yun-api-version", "v1")
            .header("x-yun-app-channel", "10000034");

        req
    }

    /// 发送 JSON 请求
    async fn json_request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: impl Serialize,
    ) -> Result<T, McloudError> {
        let response = self.request(method, path).json(&body).send().await?;
        self.handle_response(response).await
    }

    /// 处理响应
    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, McloudError> {
        let status = response.status();

        if status == StatusCode::UNAUTHORIZED {
            return Err(McloudError::TokenExpired);
        }

        let text = response
            .text()
            .await
            .map_err(|e| McloudError::ApiError(format!("读取响应失败：{}", e)))?;

        if !status.is_success() {
            return Err(McloudError::ApiError(format!("HTTP {}: {}", status, text)));
        }

        let api_response: ApiResponse<T> = serde_json::from_str(&text).map_err(|e| {
            McloudError::ParseError(format!("JSON 解析失败：{} - 响应：{}", e, text))
        })?;

        api_response.into_result().map_err(McloudError::ApiError)
    }

    /// 列出文件
    pub async fn list_files_internal(
        &self,
        parent_file_id: &str,
        page_size: u32,
        page_cursor: Option<String>,
    ) -> Result<FileListResponse, McloudError> {
        #[allow(non_snake_case)]
        #[derive(Serialize)]
        struct ListRequest {
            pageInfo: PageInfo,
            orderBy: &'static str,
            orderDirection: &'static str,
            parentFileId: String,
            imageThumbnailStyleList: Vec<&'static str>,
        }
        #[allow(non_snake_case)]
        #[derive(Serialize)]
        struct PageInfo {
            pageSize: u32,
            pageCursor: Option<String>,
        }

        let request = ListRequest {
            pageInfo: PageInfo {
                pageSize: page_size,
                pageCursor: page_cursor,
            },
            orderBy: "updated_at",
            orderDirection: "DESC",
            parentFileId: parent_file_id.to_string(),
            imageThumbnailStyleList: vec!["Small", "Large"],
        };

        self.json_request(Method::POST, "/file/list", &request)
            .await
    }

    /// 获取文件元数据
    pub async fn get_file_meta(&self, file_id: &str) -> Result<McloudFileMeta, McloudError> {
        // 通过列表获取单个文件
        let list = self.list_files_internal(file_id, 1, None).await?;

        if let Some(file) = list.files().into_iter().next() {
            Ok(file.clone())
        } else {
            Err(McloudError::NotFound("文件不存在".to_string()))
        }
    }

    /// 获取下载链接
    pub async fn get_download_url(&self, file_id: &str) -> Result<String, McloudError> {
        #[allow(non_snake_case)]
        #[derive(Serialize)]
        struct DownloadRequest {
            fileId: String,
        }

        #[derive(Deserialize)]
        struct DownloadResponse {
            data: DownloadData,
        }
        #[allow(non_snake_case)]
        #[derive(Deserialize)]
        struct DownloadData {
            downloadUrl: String,
        }

        let request = DownloadRequest {
            fileId: file_id.to_string(),
        };

        let response: DownloadResponse = self
            .json_request(Method::POST, "/file/getDownloadUrl", &request)
            .await?;

        Ok(response.data.downloadUrl)
    }

    /// 下载文件
    pub async fn download_file(&self, url: &str) -> Result<Bytes, McloudError> {
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(McloudError::DownloadError(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        Ok(response.bytes().await?)
    }

    /// 下载文件范围
    pub async fn download_file_range(
        &self,
        url: &str,
        offset: u64,
        size: Option<u64>,
    ) -> Result<Bytes, McloudError> {
        let mut request = self.client.get(url);

        let range = if let Some(len) = size {
            format!("bytes={}-{}", offset, offset + len - 1)
        } else {
            format!("bytes={}-", offset)
        };

        request = request.header("Range", range);

        let response = request.send().await?;

        if !response.status().is_success() && response.status() != StatusCode::PARTIAL_CONTENT {
            return Err(McloudError::DownloadError(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        Ok(response.bytes().await?)
    }

    /// 创建目录
    pub async fn create_folder(
        &self,
        parent_file_id: &str,
        name: &str,
    ) -> Result<McloudFileMeta, McloudError> {
        #[allow(non_snake_case)]
        #[derive(Serialize)]
        struct CreateFolderRequest {
            parentFileId: String,
            name: String,
            r#type: &'static str,
        }

        let request = CreateFolderRequest {
            parentFileId: parent_file_id.to_string(),
            name: name.to_string(),
            r#type: "folder",
        };

        #[derive(Deserialize)]
        struct CreateFolderResponse {
            data: FolderData,
        }
        #[allow(non_snake_case)]
        #[derive(Deserialize)]
        struct FolderData {
            fileId: String,
            fileName: String,
        }

        let response: CreateFolderResponse = self
            .json_request(Method::POST, "/file/create", &request)
            .await?;

        Ok(McloudFileMeta {
            id: response.data.fileId,
            name: response.data.fileName,
            file_type: McloudFileType::Folder,
            size: Some(0),
            updated_at: None,
        })
    }

    /// 删除文件
    pub async fn delete_file(&self, file_ids: Vec<String>) -> Result<(), McloudError> {
        #[allow(non_snake_case)]
        #[derive(Serialize)]
        struct DeleteRequest {
            fileIds: Vec<String>,
        }

        let request = DeleteRequest { fileIds: file_ids };

        self.json_request::<serde_json::Value>(Method::POST, "/file/batchTrash", &request)
            .await?;

        Ok(())
    }

    /// 重命名文件
    pub async fn rename_file(&self, file_id: &str, new_name: &str) -> Result<(), McloudError> {
        #[allow(non_snake_case)]
        #[derive(Serialize)]
        struct RenameRequest {
            fileId: String,
            name: String,
        }

        let request = RenameRequest {
            fileId: file_id.to_string(),
            name: new_name.to_string(),
        };

        self.json_request::<serde_json::Value>(Method::POST, "/file/update", &request)
            .await?;

        Ok(())
    }

    /// 复制文件
    pub async fn copy_file(
        &self,
        file_ids: Vec<String>,
        to_parent_file_id: &str,
    ) -> Result<(), McloudError> {
        #[allow(non_snake_case)]
        #[derive(Serialize)]
        struct CopyRequest {
            fileIds: Vec<String>,
            toParentFileId: String,
        }

        let request = CopyRequest {
            fileIds: file_ids,
            toParentFileId: to_parent_file_id.to_string(),
        };

        self.json_request::<serde_json::Value>(Method::POST, "/file/batchCopy", &request)
            .await?;

        Ok(())
    }

    // ============== 缓存管理方法 ==============

    /// 规范化路径（确保以 / 开头）
    fn normalize_path(path: &str) -> String {
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", path)
        }
    }

    /// 从缓存获取 file_id
    async fn get_file_id_by_path(&self, path: &str) -> Option<String> {
        let normalized = Self::normalize_path(path);
        self.path_cache.get_file_id(&normalized).await
    }

    /// 更新缓存
    async fn update_cache(&self, path: &str, entry: PathCacheEntry) {
        self.path_cache.insert(path, entry).await;
    }

    /// 批量更新缓存（用于列表操作）
    async fn update_cache_batch(&self, entries: Vec<(String, PathCacheEntry)>) {
        for (path, entry) in entries {
            self.path_cache.insert(&path, entry).await;
        }
    }

    /// 从缓存中移除路径
    async fn remove_cache(&self, path: &str) {
        let normalized = Self::normalize_path(path);
        self.path_cache.remove(&normalized).await;
    }

    /// 清除缓存
    async fn clear_cache(&self) {
        self.path_cache.clear().await;
    }
}

// ============== 统一的 Storage trait 实现 ==============

use std::future::Future;

/// 云盘文件内容读取器
pub struct McloudFileReader {
    url: String,
    client: Arc<Client>,
    size: Option<u64>,
    offset: u64,
}

impl McloudFileReader {
    pub fn new(url: String, size: Option<u64>, client: Arc<Client>) -> Self {
        Self {
            url,
            client,
            size,
            offset: 0,
        }
    }
}

impl AsyncRead for McloudFileReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        // 简单实现，实际应该使用流式读取
        // TODO: 实现真正的 HTTP 流式读取
        Poll::Ready(Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "流式读取暂未实现",
        )))
    }
}

impl AsyncSeek for McloudFileReader {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> std::io::Result<()> {
        let this = self.get_mut();
        let new_offset = match position {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => {
                if let Some(size) = this.size {
                    (size as i64 + offset) as u64
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "无法从末尾 seek，文件大小未知",
                    ));
                }
            }
            SeekFrom::Current(offset) => (this.offset as i64 + offset) as u64,
        };
        this.offset = new_offset;
        Ok(())
    }

    fn poll_complete(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<u64>> {
        Poll::Ready(Ok(self.get_mut().offset))
    }
}

impl FileContent for McloudFileReader {
    fn size(&self) -> Option<u64> {
        self.size
    }
}

impl crate::storage::model::Storage for McloudStorage {
    type Error = McloudError;

    fn name(&self) -> &str {
        "中国移动云盘"
    }

    fn driver_name(&self) -> &str {
        "mcloud"
    }

    fn is_readonly(&self) -> bool {
        false
    }

    fn list_files(
        &self,
        path: &str,
        page_size: u32,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<FileList, Self::Error>> + Send {
        async move {
            // 路径转换为 file_id
            let file_id = if path == "/" || path.is_empty() || path == "root" {
                "root".to_string()
            } else if let Some(id) = self.get_file_id_by_path(path).await {
                id
            } else {
                return Err(McloudError::NotFound(format!("路径不存在：{}", path)));
            };

            let response = self
                .list_files_internal(&file_id, page_size, cursor)
                .await?;

            // 更新缓存
            let parent_path = if path == "root" {
                "/".to_string()
            } else {
                Self::normalize_path(path)
            };
            let entries: Vec<(String, PathCacheEntry)> = response
                .items
                .iter()
                .map(|item| {
                    let item_path = if parent_path == "/" {
                        format!("/{}", item.name)
                    } else {
                        format!("{}/{}", parent_path, item.name)
                    };
                    let entry = PathCacheEntry {
                        file_id: item.id.clone(),
                        parent_id: file_id.clone(),
                        name: item.name.clone(),
                    };
                    (item_path.clone(), entry)
                })
                .collect();
            self.update_cache_batch(entries).await;

            Ok(response.to_file_list())
        }
    }

    fn get_meta(&self, path: &str) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send {
        async move {
            let file_id = if let Some(id) = self.get_file_id_by_path(path).await {
                id
            } else if path == "/" || path.is_empty() {
                "root".to_string()
            } else {
                "root".to_string()
            };

            let meta = self.get_file_meta(&file_id).await?;

            // 更新缓存
            let normalized = Self::normalize_path(path);
            let entry = PathCacheEntry {
                file_id: meta.id.clone(),
                parent_id: "root".to_string(),
                name: meta.name.clone(),
            };
            self.update_cache(&normalized, entry).await;

            Ok(meta.to_meta())
        }
    }

    fn get_download_url(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<String, Self::Error>> + Send {
        async move {
            let file_id = if let Some(id) = self.get_file_id_by_path(path).await {
                id
            } else if path == "/" || path.is_empty() {
                "root".to_string()
            } else {
                path.to_string()
            };
            self.get_download_url(&file_id).await
        }
    }

    fn download_file(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<Box<dyn FileContent>, Self::Error>> + Send {
        async move {
            let file_id = if let Some(id) = self.get_file_id_by_path(path).await {
                id
            } else if path == "/" || path.is_empty() {
                "root".to_string()
            } else {
                path.to_string()
            };
            let url = self.get_download_url(&file_id).await?;
            let meta = self.get_file_meta(&file_id).await?;
            let reader: Box<dyn FileContent> = Box::new(McloudFileReader::new(
                url,
                Some(meta.size.unwrap_or(0)),
                self.client.clone(),
            ));
            Ok(reader)
        }
    }

    fn create_folder(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send {
        async move {
            let normalized = Self::normalize_path(path);
            let parts: Vec<&str> = normalized.trim_start_matches('/').split('/').collect();

            let (parent_id, folder_name) = if parts.len() >= 2 {
                let parent_path = parts[..parts.len() - 1].join("/");
                let parent_id = self
                    .get_file_id_by_path(&parent_path)
                    .await
                    .unwrap_or_else(|| "root".to_string());
                (parent_id, parts.last().unwrap_or(&"").to_string())
            } else {
                ("root".to_string(), parts.last().unwrap_or(&"").to_string())
            };

            let meta = self.create_folder(&parent_id, &folder_name).await?;

            // 更新缓存
            let entry = PathCacheEntry {
                file_id: meta.id.clone(),
                parent_id,
                name: meta.name.clone(),
            };
            self.update_cache(&normalized, entry).await;

            Ok(meta.to_meta())
        }
    }

    fn delete(&self, path: &str) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            let file_id = if let Some(id) = self.get_file_id_by_path(path).await {
                id
            } else if path == "/" || path.is_empty() {
                return Err(McloudError::ApiError("不能删除根目录".to_string()));
            } else {
                path.to_string()
            };

            self.delete_file(vec![file_id]).await?;

            // 清除缓存
            self.remove_cache(path).await;

            Ok(())
        }
    }

    fn rename(
        &self,
        old_path: &str,
        new_name: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send {
        async move {
            let file_id = if let Some(id) = self.get_file_id_by_path(old_path).await {
                id
            } else {
                old_path.to_string()
            };

            self.rename_file(&file_id, new_name).await?;

            // 清除旧缓存
            self.remove_cache(old_path).await;

            // 重新获取并更新缓存
            let meta = self.get_file_meta(&file_id).await?;
            Ok(meta.to_meta())
        }
    }

    fn copy(
        &self,
        source_path: &str,
        dest_path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send {
        async move {
            let source_id = if let Some(id) = self.get_file_id_by_path(source_path).await {
                id
            } else {
                source_path.to_string()
            };

            let dest_id = if let Some(id) = self.get_file_id_by_path(dest_path).await {
                id
            } else {
                "root".to_string()
            };

            let source_id_clone = source_id.clone();

            self.copy_file(vec![source_id], &dest_id).await?;

            let meta = self.get_file_meta(&source_id_clone).await?;
            Ok(meta.to_meta())
        }
    }

    fn move_(
        &self,
        source_path: &str,
        dest_path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send {
        // TODO: 实现移动操作（需要先复制再删除）
        async move { Err(McloudError::ApiError("移动操作暂未实现".to_string())) }
    }

    fn upload_file(
        &self,
        path: &str,
        content: Vec<u8>,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send {
        // TODO: 实现上传操作
        async move { Err(McloudError::ApiError("上传操作暂未实现".to_string())) }
    }

    fn from_auth_data(json: &str) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        <McloudStorage as StorageDriver>::from_auth_data(json)
    }

    fn auth_template() -> String
    where
        Self: Sized,
    {
        <McloudStorage as StorageDriver>::auth_template()
    }
}
