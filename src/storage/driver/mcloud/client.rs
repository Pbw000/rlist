//! 中国移动云盘 API 客户端
use std::sync::Arc;

use crate::storage::driver::mcloud::config::McloudConfig;
use crate::storage::driver::mcloud::error::McloudError;
use crate::storage::driver::mcloud::types::*;
use crate::storage::file_meta::DownloadableMeta;
use crate::storage::model::{FileContent, FileList, FileMeta, Storage};
use crate::storage::radix_tree::RadixTree;
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::SeekFrom;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncSeek};
use tokio::sync::RwLock;

/// API 端点
const API_BASE: &str = "https://personal-kd-njs.yun.139.com";
const PERSONAL_NEW_BASE: &str = "/hcy";

/// 缓存条目 - 存储 file_id 和文件类型
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub file_id: String,
    pub file_meta: FileMeta,
}

impl CacheEntry {
    pub fn new(file_id: String, meta: FileMeta) -> Self {
        Self {
            file_id,
            file_meta: meta,
        }
    }
}
#[derive(Debug)]
pub struct McloudStorage {
    config: McloudConfig,
    client: Arc<Client>,
    /// 缓存 path -> file_id
    path_cache: RwLock<RadixTree<CacheEntry>>,
}
impl Clone for McloudStorage {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            client: Arc::clone(&self.client),
            path_cache: RwLock::new(RadixTree::new()),
        }
    }
}

impl McloudStorage {
    pub fn from_authorization(authorization: impl Into<String>) -> Self {
        let authorization = authorization.into();
        let config = McloudConfig { authorization };
        Self {
            client: Arc::new(Client::new()),
            config,
            path_cache: RwLock::new(RadixTree::new()),
        }
    }
}

impl Storage for McloudStorage {
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

    async fn build_cache(&self) -> Result<(), McloudError> {
        self.build_cache_recursive("root", "/".to_string()).await
    }

    async fn handle_path(&self, path: &str) -> Result<FileMeta, Self::Error> {
        let file_id = self
            .get_file_id_by_path(path)
            .await
            .ok_or(McloudError::NotFound("File not found in cache".to_string()))?;
        dbg!(&file_id);
        let meta = self.get_file_meta(&file_id).await?;
        Ok(meta.to_meta())
    }

    fn list_files(
        &self,
        path: &str,
        page_size: u32,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<FileList, Self::Error>> + Send {
        async move {
            // 从缓存获取子目录列表
            {
                let cache = self.path_cache.read().await;
                let children = cache.search_children(path);
                if !children.is_empty() && cursor.is_none() {
                    // 有缓存且是第一页，直接返回缓存
                    let mut items = Vec::new();
                    for (_, child_node) in children {
                        if let Some(cache_entry) = &child_node.value {
                            items.push(cache_entry.file_meta.clone());
                        }
                    }
                    return Ok(FileList {
                        total: children.len() as u64,
                        items,
                        next_cursor: None,
                    });
                }
            }
            // 首先尝试从缓存获取 file_id
            let file_id = if path == "/" || path.is_empty() || path == "root" {
                "root".to_string()
            } else if let Some(id) = self.get_file_id_by_path(path).await {
                id
            } else {
                return Err(McloudError::NotFound("File not found in cache".to_string()));
            };

            // 缓存未命中或需要分页，调用 API
            let response = self
                .list_files_internal(&file_id, page_size, cursor.clone())
                .await?;

            // 更新路径缓存
            let parent_path = if path == "root" { "/" } else { path };
            let entries: Vec<(String, CacheEntry)> = response
                .items
                .iter()
                .map(|item| {
                    let item_path = if parent_path == "/" {
                        format!("/{}", item.name)
                    } else {
                        format!("{}/{}", parent_path, item.name)
                    };
                    (item_path, CacheEntry::new(item.id.clone(), item.to_meta()))
                })
                .collect();
            self.update_cache_batch(entries).await;

            Ok(response.to_file_list())
        }
    }
    async fn get_meta(&self, path: &str) -> Result<FileMeta, Self::Error> {
        let file_id = self
            .get_file_id_by_path(path)
            .await
            .ok_or(McloudError::NotFound("File not found in cache".to_string()))?;

        let meta = self.get_file_meta(&file_id).await?;
        Ok(meta.to_meta())
    }

    async fn get_download_meta_by_path(&self, path: &str) -> Result<DownloadableMeta, Self::Error> {
        let file_id = self
            .get_file_id_by_path(path)
            .await
            .ok_or(McloudError::DownloadError("No cache hit!".to_owned()))?;
        self.get_download_url_by_file_id(&file_id).await
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
            let meta = self.get_download_meta_by_path(&file_id).await?;
            let reader: Box<dyn FileContent> = Box::new(McloudFileReader::new(
                meta.download_url,
                Some(meta.size),
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
            let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

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
            self.update_cache(path, CacheEntry::new(meta.id.clone(), meta.to_meta()))
                .await;

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

            // 获取父目录 file_id 以使缓存失效
            let parent_path = old_path.rsplit_once('/').map(|(p, _)| p).unwrap_or("/");
            let _parent_file_id = self.get_file_id_by_path(parent_path).await;

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
        _source_path: &str,
        _dest_path: &str,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send {
        // TODO: 实现移动操作（需要先复制再删除）
        async move { Err(McloudError::ApiError("移动操作暂未实现".to_string())) }
    }

    fn upload_file(
        &self,
        _path: &str,
        _content: Vec<u8>,
    ) -> impl Future<Output = Result<FileMeta, Self::Error>> + Send {
        // TODO: 实现上传操作
        async move { Err(McloudError::ApiError("上传操作暂未实现".to_string())) }
    }

    fn upload_mode(&self) -> crate::storage::model::UploadMode {
        crate::storage::model::UploadMode::Relay
    }

    fn get_upload_info(
        &self,
        _path: &str,
        _size: u64,
    ) -> impl Future<Output = Result<crate::storage::model::UploadInfo, Self::Error>> + Send {
        async move {
            // 移动云暂不支持 Direct 上传模式
            Err(McloudError::ApiError("当前存储不支持 Direct 上传模式".to_string()).into())
        }
    }

    fn from_auth_data(json: &str) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        #[derive(serde::Deserialize)]
        struct AuthJson {
            authorization: String,
        }

        let auth_json: AuthJson = serde_json::from_str(json)
            .map_err(|_e| McloudError::ParseError("认证数据解析失败".to_string()))?;

        Ok(Self::from_authorization(auth_json.authorization))
    }

    fn auth_template(&self) -> String
    where
        Self: Sized,
    {
        r#"{"type": "token", "fields": ["authorization"]}"#.to_string()
    }
}

impl McloudStorage {
    pub fn new(config: McloudConfig) -> Self {
        Self {
            config,
            client: Arc::new(Client::new()),
            path_cache: RwLock::new(RadixTree::new()),
        }
    }

    pub fn with_client_arc(config: McloudConfig, client: Arc<Client>) -> Self {
        Self {
            config,
            client,
            path_cache: RwLock::new(RadixTree::new()),
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
    pub async fn get_download_url_by_file_id(
        &self,
        file_id: impl Into<String>,
    ) -> Result<DownloadableMeta, McloudError> {
        #[allow(non_snake_case)]
        #[derive(Serialize)]
        struct DownloadRequest {
            fileId: String,
        }

        let request = DownloadRequest {
            fileId: file_id.into(),
        };

        let response = self
            .request(Method::POST, "/file/getDownloadUrl")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(McloudError::TokenExpired);
        }
        #[allow(non_snake_case)]
        #[derive(Deserialize)]
        struct DownloadMeta {
            url: String,
            size: u64,
            contentHash: String,
        }

        let download_meta: ApiResponse<DownloadMeta> = response
            .json()
            .await
            .map_err(|e| McloudError::ApiError(format!("读取下载元数据失败：{}", e)))?;
        let download_meta = download_meta
            .into_result()
            .map_err(|e| McloudError::ApiError(e))?;
        Ok(DownloadableMeta {
            download_url: download_meta.url,
            size: download_meta.size,
            hash: download_meta.contentHash,
        })
    }

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

        self.json_request::<serde_json::Value>(Method::POST, "/recyclebin/batchTrash", &request)
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

    async fn get_file_id_by_path(&self, path: &str) -> Option<String> {
        let cache = self.path_cache.read().await;
        let matched = cache.search(path)?;
        if matched.1.is_empty() {
            //exact match
            Some(matched.0.file_id.to_string())
        } else {
            None
        }
    }

    /// 更新缓存
    async fn update_cache(&self, path: &str, cache_entry: CacheEntry) {
        let mut cache = self.path_cache.write().await;
        cache.insert(path, cache_entry);
    }

    async fn update_cache_batch(&self, entries: Vec<(String, CacheEntry)>) {
        let mut cache = self.path_cache.write().await;
        for (path, cache_entry) in entries {
            cache.insert(&path, cache_entry);
        }
    }

    /// 从缓存中移除路径
    async fn remove_cache(&self, path: &str) {
        let mut cache = self.path_cache.write().await;
        cache.remove(path);
    }

    /// 清除缓存
    async fn clear_cache(&self) {
        let mut cache = self.path_cache.write().await;
        cache.clear();
    }

    async fn build_cache_recursive(&self, file_id: &str, path: String) -> Result<(), McloudError> {
        let mut all_entries = Vec::new();
        let mut cursor = None;

        loop {
            let response = self.list_files_internal(file_id, 100, cursor).await?;

            // 收集条目
            for item in &response.items {
                let item_path = if path == "/" {
                    format!("/{}", item.name)
                } else {
                    format!("{}/{}", path, item.name)
                };

                all_entries.push((
                    item_path.clone(),
                    CacheEntry::new(item.id.clone(), item.to_meta()),
                ));
                if item.file_type == McloudFileType::Folder {
                    Box::pin(self.build_cache_recursive(&item.id, item_path)).await?;
                }
            }
            if response.hasMore.unwrap_or(false) {
                cursor = response.nextPageCursor;
            } else {
                break;
            }
        }
        // 批量更新缓存
        self.update_cache_batch(all_entries).await;

        Ok(())
    }
}

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
