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
    pub file_meta: McloudFileMeta,
}

impl CacheEntry {
    pub fn new(meta: McloudFileMeta) -> Self {
        Self { file_meta: meta }
    }
    pub fn file_id(&self) -> &str {
        &self.file_meta.id
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

    async fn build_cache(&self, path: &str) -> Result<(), McloudError> {
        if path.is_empty() || path == "/" {
            // 根路径，直接构建
            self.build_cache_recursive("root", "/".to_string()).await?;
            return Ok(());
        }

        let cache = self.path_cache.read().await;
        // 搜索最长公共前缀
        if let Some((cached_entry, remainder)) = cache.search(path) {
            // remainder 是未匹配的后缀部分，如 "/subdir1/subdir2"
            let remainder = remainder.trim_start_matches('/');
            if remainder.is_empty() {
                // 完全匹配，无需构建
                return Ok(());
            }

            // 克隆 file_id 以避免借用问题
            let ancestor_file_id = cached_entry.file_id().to_string();
            drop(cache);
            // dbg!(&ancestor_file_id);
            self.build_cache_from_ancestor(&ancestor_file_id, path.to_string(), remainder)
                .await?;
        } else {
            // 没有匹配任何缓存，从 root 开始构建
            drop(cache);
            self.build_cache_recursive("root", "/".to_string()).await?;
        }

        Ok(())
    }

    async fn handle_path(&self, path: &str) -> Result<FileMeta, Self::Error> {
        let meta = self.get_meta(&path).await?;
        Ok(meta)
    }

    fn list_files(
        &self,
        path: &str,
        page_size: u32,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<FileList, Self::Error>> + Send {
        // dbg!(&path);

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
                            items.push(cache_entry.file_meta.to_meta());
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
                    (item_path, CacheEntry::new(item.clone()))
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

        let meta = self.get_file_meta_by_path(&file_id).await?;
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
                return Err(McloudError::NotFound("File not found!".into()));
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
            self.update_cache(path, CacheEntry::new(meta.clone())).await;

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

            let meta = self.get_file_meta_by_path(&parent_path).await?;
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

            let meta = self.get_file_meta_by_path(&source_id_clone).await?;
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

    async fn upload_file<R: tokio::io::AsyncRead + Send + Unpin + 'static>(
        &self,
        _path: &str,
        content: R,
        param: crate::storage::model::UploadInfoParams,
    ) -> Result<FileMeta, Self::Error> {
        // 解析路径，获取文件名和父目录
        let path = &param.path;
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let file_name = parts
            .last()
            .ok_or(McloudError::ApiError("无效的文件路径".to_string()))?;

        // 获取父目录 file_id
        let parent_file_id = if parts.len() >= 2 {
            let parent_path = parts[..parts.len() - 1].join("/");
            self.get_file_id_by_path(&parent_path)
                .await
                .unwrap_or_else(|| "root".to_string())
        } else {
            "root".to_string()
        };

        let file_size = param.size;
        // dbg!(&param);
        // 1. 创建文件记录
        let create_result = self
            .create_upload_record(&parent_file_id, file_name, file_size, &param.hash)
            .await?;

        if !create_result.upload_url.is_empty() {
            self.upload_to_eos(&create_result.upload_url, content)
                .await?;

            // 3. 提交上传
            self.complete_upload(
                path,
                &create_result.upload_id,
                &create_result.file_id,
                &param.hash,
            )
            .await?
            .ok_or_else(|| McloudError::ApiError("complete upload returned None".to_string()))?;
        };

        // 更新缓存
        let file_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };
        self.build_cache(&file_path).await?;
        self.get_meta(path).await
    }

    fn upload_mode(&self) -> crate::storage::model::UploadMode {
        crate::storage::model::UploadMode::Direct
    }

    fn get_upload_info(
        &self,
        params: crate::storage::model::UploadInfoParams,
    ) -> impl Future<Output = Result<crate::storage::model::UploadInfo, Self::Error>> + Send {
        async move {
            let parts: Vec<&str> = params.path.trim_start_matches('/').split('/').collect();
            let file_name = parts
                .last()
                .ok_or(McloudError::ApiError("无效的文件路径".to_string()))?;
            let parent_file_id = if parts.len() >= 2 {
                let parent_path = parts[..parts.len() - 1].join("/");
                let _ = self.build_cache(&parent_path).await;

                // 从缓存获取父目录 file_id
                self.get_file_id_by_path(&parent_path)
                    .await
                    .unwrap_or_else(|| "root".to_string())
            } else {
                "root".to_string()
            };

            // 1. 创建文件记录，获取上传 URL
            let create_result = self
                .create_upload_record(&parent_file_id, file_name, params.size, &params.hash)
                .await?;

            // 检查是否是秒传
            if create_result.upload_url.is_empty() {
                self.build_cache(&params.path).await.ok();
                Ok(crate::storage::model::UploadInfo {
                    upload_url: "about:blank".to_string(),
                    method: "POST".to_string(),
                    form_fields: None,
                    headers: None,
                    complete_url: None,
                })
            } else {
                // 构建上传请求头
                let mut headers = std::collections::HashMap::new();
                headers.insert(
                    "Content-Type".to_string(),
                    "application/octet-stream".to_string(),
                );
                headers.insert("Origin".to_string(), "https://yun.139.com".to_string());
                headers.insert("Referer".to_string(), "https://yun.139.com/".to_string());
                headers.insert("User-Agent".to_string(), "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36 Edg/146.0.0.0".to_string());

                // mcloud 使用 PUT 方法直接上传文件内容，不需要 form_fields
                // form_fields 用于 S3 等需要表单字段的存储

                Ok(crate::storage::model::UploadInfo {
                    upload_url: create_result.upload_url,
                    method: "PUT".to_string(),
                    form_fields: None, // mcloud 不需要表单字段
                    headers: Some(headers),
                    complete_url: Some(format!(
                        "/api/fs/upload/complete?path={}&upload_id={}&file_id={}",
                        params.path, // 使用绝对路径
                        create_result.upload_id,
                        create_result.file_id
                    )),
                })
            }
        }
    }

    async fn complete_upload(
        &self,
        path: &str,
        upload_id: &str,
        file_id: &str,
        content_hash: &str,
    ) -> Result<Option<crate::storage::model::FileMeta>, Self::Error> {
        // 调用 /hcy/file/complete API
        #[derive(Serialize)]
        struct CompleteRequest {
            fileId: String,
            uploadId: String,
            contentHash: String,
            contentHashAlgorithm: &'static str,
        }

        let request = CompleteRequest {
            fileId: file_id.to_string(),
            uploadId: upload_id.to_string(),
            contentHash: content_hash.to_string(),
            contentHashAlgorithm: "SHA256",
        };

        #[derive(Deserialize)]
        struct CompleteData {
            fileId: String,
            name: String,
            r#type: String,
        }

        let _: CompleteData = self
            .json_request(Method::POST, "/file/complete", &request)
            .await?;

        // 更新缓存
        let file_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };
        self.build_cache(&file_path).await?;
        Ok(self
            .get_file_meta_by_path(&file_path)
            .await
            .ok()
            .and_then(|v| Some(v.to_meta())))
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

    /// 处理 API 响应并返回 ApiResponse 包装的类型
    async fn json_request_with_response<T: for<'de> DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: impl Serialize,
    ) -> Result<T, McloudError> {
        let response = self.request(method, path).json(&body).send().await?;
        self.handle_api_response(response).await
    }

    /// 处理响应（直接返回数据）
    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, McloudError> {
        let status = response.status();

        if status == StatusCode::UNAUTHORIZED {
            return Err(McloudError::TokenExpired("HTTP 401 未授权".to_string()));
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

    async fn handle_api_response<T: for<'de> DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, McloudError> {
        let status = response.status();

        if status == StatusCode::UNAUTHORIZED {
            return Err(McloudError::TokenExpired("HTTP 401 未授权".to_string()));
        }

        let text = response
            .text()
            .await
            .map_err(|e| McloudError::ApiError(format!("读取响应失败：{}", e)))?;

        if !status.is_success() {
            return Err(McloudError::ApiError(format!("HTTP {}: {}", status, text)));
        }

        // 尝试解析为 ApiResponse<T> 并提取 data
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
    pub async fn get_file_meta_by_path(&self, path: &str) -> Result<McloudFileMeta, McloudError> {
        let cache = self.path_cache.read().await;
        if let Some((entry, _)) = cache.search(path) {
            return Ok(entry.file_meta.clone());
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
            return Err(McloudError::TokenExpired("HTTP 401 未授权".to_string()));
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

        #[allow(non_snake_case)]
        #[derive(Deserialize)]
        struct FolderData {
            fileId: String,
            fileName: String,
        }

        let response: FolderData = self
            .json_request_with_response(Method::POST, "/file/create", &request)
            .await?;

        Ok(McloudFileMeta {
            id: response.fileId,
            name: response.fileName,
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
            Some(matched.0.file_id().to_string())
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

    /// 从已缓存的祖先节点开始构建缓存
    /// `target_path` 是目标完整路径，`remainder` 是未匹配的后缀部分（如 "subdir1/subdir2"）
    async fn build_cache_from_ancestor(
        &self,
        ancestor_file_id: &str,
        target_path: String,
        remainder: &str,
    ) -> Result<(), McloudError> {
        // 计算祖先路径
        let ancestor_path = if target_path.ends_with(remainder) && !remainder.is_empty() {
            let end = target_path.len() - remainder.len();
            target_path[..end].trim_end_matches('/').to_string()
        } else {
            String::new()
        };
        let ancestor_path = if ancestor_path.is_empty() {
            "/".to_string()
        } else {
            ancestor_path
        };

        // 逐层构建剩余路径
        let mut current_file_id = ancestor_file_id.to_string();
        let mut current_path = ancestor_path;
        let mut remaining_parts: Vec<&str> =
            remainder.split('/').filter(|s| !s.is_empty()).collect();

        while !remaining_parts.is_empty() {
            let target_name = remaining_parts.remove(0);

            // 获取当前目录下的所有子项
            let mut cursor = None;
            let mut found = false;

            loop {
                let response = self
                    .list_files_internal(&current_file_id, 100, cursor)
                    .await?;

                // 查找目标子项
                for item in &response.items {
                    if item.name == target_name {
                        // 找到目标，更新当前路径和 file_id
                        current_path = if current_path == "/" {
                            format!("/{}", item.name)
                        } else {
                            format!("{}/{}", current_path, item.name)
                        };
                        current_file_id = item.id.clone();
                        found = true;

                        // 如果是文件夹，构建其完整缓存
                        if item.file_type == McloudFileType::Folder {
                            // 递归构建子目录缓存
                            self.build_cache_recursive(&item.id, current_path.clone())
                                .await?;
                        }
                        break;
                    }
                }

                if found {
                    break;
                }

                if response.hasMore.unwrap_or(false) {
                    cursor = response.nextPageCursor;
                } else {
                    // 没有找到目标，提前退出
                    return Err(McloudError::NotFound(format!(
                        "Path '{}' not found",
                        target_path
                    )));
                }
            }
        }

        Ok(())
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

                all_entries.push((item_path.clone(), CacheEntry::new(item.clone())));
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

    /// 创建上传记录，返回上传信息
    async fn create_upload_record(
        &self,
        parent_file_id: &str,
        file_name: &str,
        file_size: u64,
        content_hash: &str,
    ) -> Result<UploadCreateInfo, McloudError> {
        #[derive(Serialize)]
        struct CreateUploadRequest {
            fileRenameMode: &'static str,
            contentType: &'static str,
            r#type: &'static str,
            name: String,
            size: u64,
            contentHashAlgorithm: &'static str,
            contentHash: String,
            partInfos: Vec<PartInfo>,
            parentFileId: String,
        }

        #[derive(Serialize)]
        struct PartInfo {
            parallelHashCtx: ParallelHashCtx,
            partNumber: u32,
            partSize: u64,
        }

        #[derive(Serialize)]
        struct ParallelHashCtx {
            partOffset: u64,
        }

        let request = CreateUploadRequest {
            fileRenameMode: "auto_rename",
            contentType: "application/octet-stream",
            r#type: "file",
            name: file_name.to_string(),
            size: file_size,
            contentHashAlgorithm: "SHA256",
            contentHash: content_hash.to_string(),
            partInfos: vec![PartInfo {
                parallelHashCtx: ParallelHashCtx { partOffset: 0 },
                partNumber: 1,
                partSize: file_size,
            }],
            parentFileId: parent_file_id.to_string(),
        };

        #[derive(Deserialize, Debug)]
        struct CreateUploadData {
            fileId: String,
            uploadId: Option<String>,
            partInfos: Option<Vec<PartUploadInfo>>,
            rapidUpload: Option<bool>,
            exist: Option<bool>,
        }

        #[derive(Deserialize, Debug)]
        struct PartUploadInfo {
            uploadUrl: Option<String>,
        }

        let response: CreateUploadData = self
            .json_request_with_response(Method::POST, "/file/create", &request)
            .await?;
        // dbg!(&response);

        // 检查是否是秒传（hash 命中缓存）
        if response.rapidUpload == Some(true)
            || response.exist == Some(true)
            || response.uploadId.is_none()
            || response.partInfos.is_none()
        {
            // 秒传成功，直接返回文件 ID，无需实际上传
            return Ok(UploadCreateInfo {
                file_id: response.fileId,
                upload_id: String::new(),  // 空 upload_id 表示秒传
                upload_url: String::new(), // 空 upload_url 表示无需上传
                part_size: 0,
                part_offset: 0,
            });
        }

        let upload_url = response
            .partInfos
            .and_then(|parts| parts.first().and_then(|p| p.uploadUrl.clone()))
            .ok_or(McloudError::ApiError("未获取到上传 URL".to_string()))?;

        // part_size 和 part_offset 使用传入的文件大小和 0 偏移
        // 因为响应中 PartUploadInfo 只包含 uploadUrl
        let part_size = file_size;
        let part_offset = 0u64;

        Ok(UploadCreateInfo {
            file_id: response.fileId,
            upload_id: response.uploadId.unwrap_or_default(),
            upload_url,
            part_size,
            part_offset,
        })
    }

    /// 上传文件内容到 EOS 存储（流式上传）
    async fn upload_to_eos(
        &self,
        upload_url: &str,
        content: impl tokio::io::AsyncRead + Unpin + Send + 'static,
    ) -> Result<(), McloudError> {
        use tokio_util::io::ReaderStream;

        // 将 AsyncRead 转换为 Stream
        let stream = ReaderStream::new(content);

        // 使用 reqwest::Body::wrap_stream 包装
        let body = reqwest::Body::wrap_stream(stream);

        let response = self
            .client
            .put(upload_url)
            .header("Content-Type", "application/octet-stream")
            .body(body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(McloudError::ApiError(format!(
                "上传到 EOS 失败：{}",
                error_text
            )));
        }

        Ok(())
    }
}

/// 上传创建信息
struct UploadCreateInfo {
    file_id: String,
    upload_id: String,
    upload_url: String,
    part_size: u64,
    part_offset: u64,
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
