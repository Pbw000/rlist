//! 天翼云盘 API 客户端

#![allow(dead_code)]
#![allow(non_snake_case)]

use std::sync::Arc;

use chrono::Utc;
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

use crate::storage::driver::ecloud::config::EcloudConfig;
use crate::storage::driver::ecloud::error::EcloudError;
use crate::storage::driver::ecloud::types::*;
use crate::storage::file_meta::DownloadableMeta;
use crate::storage::model::{FileContent, FileList, FileMeta, Storage};
use crate::storage::radix_tree::RadixTree;
use crate::storage::url_reader::UrlReader;

use serde::Deserialize;

/// API 端点
const API_URL: &str = "https://api.cloud.189.cn";
const UPLOAD_URL: &str = "https://upload.cloud.189.cn";

/// 缓存条目
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub file_meta: EcloudFileMeta,
}

impl CacheEntry {
    pub fn new(meta: EcloudFileMeta) -> Self {
        Self { file_meta: meta }
    }
    pub fn file_id(&self) -> &str {
        &self.file_meta.id
    }
}

#[derive(Debug)]
pub struct EcloudStorage {
    config: EcloudConfig,
    client: Arc<Client>,
    path_cache: RwLock<RadixTree<CacheEntry>>,
}

impl Clone for EcloudStorage {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            client: Arc::clone(&self.client),
            path_cache: RwLock::new(RadixTree::new()),
        }
    }
}

impl PartialEq for EcloudStorage {
    fn eq(&self, other: &Self) -> bool {
        self.config.session_key == other.config.session_key
    }
}

impl Eq for EcloudStorage {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMeta {
    pub session_key: String,
    pub session_secret: String,
}

impl Default for ConfigMeta {
    fn default() -> Self {
        Self {
            session_key: String::new(),
            session_secret: String::new(),
        }
    }
}

impl EcloudStorage {
    pub fn from_session(session_key: String, session_secret: String) -> Self {
        let config = EcloudConfig {
            session_key,
            session_secret,
        };
        Self {
            client: Arc::new(Client::new()),
            config,
            path_cache: RwLock::new(RadixTree::new()),
        }
    }
}

impl Storage for EcloudStorage {
    type Error = EcloudError;
    type End2EndCopyMeta = String;
    type End2EndMoveMeta = String;
    type ConfigMeta = ConfigMeta;

    fn name(&self) -> &str {
        "天翼云盘"
    }

    fn driver_name(&self) -> &str {
        "ecloud"
    }

    fn hash(&self) -> u64 {
        use std::hash::Hasher;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        hasher.write(self.config.session_key.as_bytes());
        hasher.finish()
    }

    async fn build_cache(&self, path: &str) -> Result<(), EcloudError> {
        if path.is_empty() || path == "/" {
            self.build_cache_recursive("-11", "/".to_string()).await?;
            return Ok(());
        }

        let cache = self.path_cache.read().await;
        if let Some((cached_entry, remainder)) = cache.search(path) {
            let remainder = remainder.trim_start_matches('/');
            if remainder.is_empty() {
                return Ok(());
            }
            let ancestor_file_id = cached_entry.file_id().to_string();
            drop(cache);
            self.build_cache_from_ancestor(&ancestor_file_id, path.to_string(), remainder)
                .await?;
        } else {
            drop(cache);
            self.build_cache_recursive("-11", "/".to_string()).await?;
        }
        Ok(())
    }

    async fn handle_path(&self, path: &str) -> Result<FileMeta, EcloudError> {
        self.get_meta(path).await
    }

    async fn list_files(
        &self,
        path: &str,
        page_size: u32,
        cursor: Option<String>,
    ) -> Result<FileList, EcloudError> {
        {
            let cache = self.path_cache.read().await;
            let children = cache.search_children(path);
            if !children.is_empty() && cursor.is_none() {
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

        let file_id = if path == "/" || path.is_empty() {
            "-11".to_string()
        } else if let Some(id) = self.get_file_id_by_path(path).await {
            id
        } else {
            return Err(EcloudError::NotFound("File not found in cache".to_string()));
        };

        let response = self
            .list_files_internal(&file_id, page_size, cursor.clone())
            .await?;

        let parent_path = if path == "/" { "" } else { path };
        let entries: Vec<(String, CacheEntry)> = response
            .file_list
            .folders
            .iter()
            .chain(response.file_list.files.iter())
            .map(|item| {
                let item_path = if parent_path.is_empty() {
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

    async fn get_meta(&self, path: &str) -> Result<FileMeta, EcloudError> {
        let meta = self.get_file_meta_by_path(path).await?;
        Ok(meta.to_meta())
    }

    async fn get_download_meta_by_path(&self, path: &str) -> Result<DownloadableMeta, EcloudError> {
        let file_id = self
            .get_file_id_by_path(path)
            .await
            .ok_or_else(|| EcloudError::DownloadError("No cache hit".to_string()))?;
        self.get_download_url_by_file_id(&file_id).await
    }

    async fn download_file(&self, path: &str) -> Result<Box<dyn FileContent>, EcloudError> {
        let meta = self.get_download_meta_by_path(path).await?;
        let reader = UrlReader::builder(&meta.download_url)
            .size(meta.size)
            .hash(&meta.hash)
            .build();
        Ok(Box::new(reader))
    }

    async fn create_folder(&self, path: &str) -> Result<FileMeta, EcloudError> {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let folder_name = parts.last().unwrap_or(&"").to_string();

        let parent_file_id = if parts.len() >= 2 {
            let parent_path = parts[..parts.len() - 1].join("/");
            self.get_file_id_by_path(&parent_path)
                .await
                .unwrap_or_else(|| "-11".to_string())
        } else {
            "-11".to_string()
        };

        let meta = self
            .create_folder_internal(&parent_file_id, &folder_name)
            .await?;

        self.update_cache(path, CacheEntry::new(meta.clone())).await;
        Ok(meta.to_meta())
    }

    async fn delete(&self, path: &str) -> Result<(), EcloudError> {
        let file_id = if let Some(id) = self.get_file_id_by_path(path).await {
            id
        } else if path == "/" || path.is_empty() {
            return Err(EcloudError::ApiError("不能删除根目录".to_string()));
        } else {
            return Err(EcloudError::NotFound("File not found".to_string()));
        };

        self.delete_file(vec![file_id.clone()]).await?;
        self.remove_cache(path).await;
        Ok(())
    }

    async fn rename(&self, old_path: &str, new_name: &str) -> Result<(), EcloudError> {
        let file_id = if let Some(id) = self.get_file_id_by_path(old_path).await {
            id
        } else {
            return Err(EcloudError::NotFound("File not found".to_string()));
        };

        let parent_path = old_path.rsplit_once('/').map(|(p, _)| p).unwrap_or("/");
        self.rename_file(&file_id, new_name).await?;
        self.remove_cache(old_path).await;
        self.build_cache(&parent_path).await?;
        Ok(())
    }

    async fn copy_end_to_end(
        &self,
        source_meta: Self::End2EndCopyMeta,
        dest_path: &str,
    ) -> Result<(), EcloudError> {
        let source_id = source_meta;
        let dest_id = if let Some(id) = self.get_file_id_by_path(dest_path).await {
            id
        } else {
            "-11".to_string()
        };

        self.copy_file(vec![source_id], &dest_id).await?;
        Ok(())
    }

    async fn gen_copy_meta(&self, path: &str) -> Result<Self::End2EndCopyMeta, EcloudError> {
        let file_id = self
            .get_file_id_by_path(path)
            .await
            .ok_or_else(|| EcloudError::NotFound("File not found in cache".to_string()))?;
        Ok(file_id)
    }

    async fn move_end_to_end(
        &self,
        source_meta: Self::End2EndMoveMeta,
        dest_path: &str,
    ) -> Result<(), EcloudError> {
        let source_id = source_meta;
        let dest_id = if let Some(id) = self.get_file_id_by_path(dest_path).await {
            id
        } else {
            "-11".to_string()
        };

        self.move_file(vec![source_id.clone()], &dest_id).await?;
        Ok(())
    }

    async fn gen_move_meta(&self, path: &str) -> Result<Self::End2EndMoveMeta, EcloudError> {
        let file_id = self
            .get_file_id_by_path(path)
            .await
            .ok_or_else(|| EcloudError::NotFound("File not found in cache".to_string()))?;
        Ok(file_id)
    }

    async fn upload_file<R: tokio::io::AsyncRead + Send + Unpin + 'static>(
        &self,
        path: &str,
        content: R,
        param: crate::storage::model::UploadInfoParams,
    ) -> Result<FileMeta, EcloudError> {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let file_name = parts
            .last()
            .ok_or_else(|| EcloudError::ApiError("无效路径".to_string()))?
            .to_string();

        let parent_file_id = if parts.len() >= 2 {
            let parent_path = parts[..parts.len() - 1].join("/");
            self.get_file_id_by_path(&parent_path)
                .await
                .unwrap_or_else(|| "-11".to_string())
        } else {
            "-11".to_string()
        };

        let upload_result = self
            .init_upload(&parent_file_id, &file_name, param.size, &param.hash)
            .await?;

        if upload_result.file_data_exists == Some(1) {
            self.build_cache(path).await?;
            return self.get_meta(path).await;
        }

        if let Some(upload_url) = upload_result.file_upload_url {
            self.upload_to_eos(&upload_url, content, param.size).await?;

            if let Some(commit_url) = upload_result.file_commit_url {
                self.commit_upload(
                    &commit_url,
                    &upload_result.upload_file_id.unwrap_or_default(),
                )
                .await?;
            }
        }

        self.build_cache(path).await?;
        self.get_meta(path).await
    }

    async fn get_upload_info(
        &self,
        params: crate::storage::model::UploadInfoParams,
    ) -> Result<crate::storage::model::UploadInfo, EcloudError> {
        let parts: Vec<&str> = params.path.trim_start_matches('/').split('/').collect();
        let file_name = parts
            .last()
            .ok_or_else(|| EcloudError::ApiError("无效路径".to_string()))?
            .to_string();

        let parent_file_id = if parts.len() >= 2 {
            let parent_path = parts[..parts.len() - 1].join("/");
            self.build_cache(&parent_path).await.ok();
            self.get_file_id_by_path(&parent_path)
                .await
                .unwrap_or_else(|| "-11".to_string())
        } else {
            "-11".to_string()
        };

        let upload_result = self
            .init_upload(&parent_file_id, &file_name, params.size, &params.hash)
            .await?;

        if upload_result.file_data_exists == Some(1) {
            self.build_cache(&params.path).await.ok();
            Ok(crate::storage::model::UploadInfo {
                upload_url: "about:blank".to_string(),
                method: "POST".to_string(),
                form_fields: None,
                headers: None,
                complete_url: None,
            })
        } else {
            let mut headers = std::collections::HashMap::new();
            headers.insert(
                "Content-Type".to_string(),
                "application/octet-stream".to_string(),
            );

            Ok(crate::storage::model::UploadInfo {
                upload_url: upload_result.file_upload_url.unwrap_or_default(),
                method: "PUT".to_string(),
                form_fields: None,
                headers: Some(headers),
                complete_url: Some(format!(
                    "/api/fs/upload/complete?path={}&upload_id={}",
                    params.path,
                    upload_result.upload_file_id.unwrap_or_default()
                )),
            })
        }
    }

    async fn complete_upload(
        &self,
        path: &str,
        upload_id: &str,
        _file_id: &str,
        _content_hash: &str,
    ) -> Result<Option<FileMeta>, EcloudError> {
        let commit_url = format!("{}/multiple/commitMultiUploadFile.action", UPLOAD_URL);

        #[derive(Serialize)]
        struct CommitRequest {
            uploadFileId: String,
        }

        let request = CommitRequest {
            uploadFileId: upload_id.to_string(),
        };

        let _: CommitUploadResp = self
            .json_request(Method::POST, &commit_url, &request)
            .await?;

        self.build_cache(path).await?;
        Ok(self
            .get_file_meta_by_path(path)
            .await
            .ok()
            .map(|m| m.to_meta()))
    }

    fn from_auth_data(data: Self::ConfigMeta) -> Result<Self, EcloudError> {
        Ok(Self::from_session(data.session_key, data.session_secret))
    }

    fn auth_template() -> Self::ConfigMeta {
        ConfigMeta {
            session_key: "your_session_key".to_string(),
            session_secret: "your_session_secret".to_string(),
        }
    }

    fn to_auth_data(&self) -> Self::ConfigMeta {
        ConfigMeta {
            session_key: self.config.session_key.clone(),
            session_secret: self.config.session_secret.clone(),
        }
    }
}

impl EcloudStorage {
    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = if path.starts_with("http") {
            path.to_string()
        } else {
            format!("{}{}", API_URL, path)
        };

        let timestamp = Utc::now().to_rfc2822();
        let request_id = uuid_simple();
        let signature = self.generate_signature(method.as_str(), &url, &timestamp);

        self.client
            .request(method, &url)
            .header("Accept", "application/json;charset=UTF-8")
            .header("Date", &timestamp)
            .header("X-Request-ID", &request_id)
            .header("SessionKey", &self.config.session_key)
            .header("Signature", &signature)
    }

    /// 生成 HMAC 签名
    fn generate_signature(&self, method: &str, _url: &str, date: &str) -> String {
        use hmac::{Hmac, Mac};
        use sha1::Sha1;

        type HmacSha1 = Hmac<Sha1>;

        let secret = &self.config.session_secret;
        let string_to_sign = format!("{}\n{}\n", method, date);

        let mut mac =
            HmacSha1::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());
        let result = mac.finalize();
        let code_bytes = result.into_bytes();

        use base64::{Engine, engine::general_purpose::STANDARD};
        STANDARD.encode(&code_bytes)
    }

    async fn json_request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: impl Serialize,
    ) -> Result<T, EcloudError> {
        let response = self.request(method, path).json(&body).send().await?;
        self.handle_response(response).await
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, EcloudError> {
        let status = response.status();

        if status == StatusCode::UNAUTHORIZED {
            return Err(EcloudError::TokenExpired("HTTP 401 未授权".to_string()));
        }

        let text = response
            .text()
            .await
            .map_err(|e| EcloudError::ApiError(format!("读取响应失败：{}", e)))?;

        if !status.is_success() {
            return Err(EcloudError::ApiError(format!("HTTP {}: {}", status, text)));
        }

        let api_response: EcloudApiResponse<T> = serde_json::from_str(&text).map_err(|e| {
            EcloudError::ParseError(format!("JSON 解析失败：{} - 响应：{}", e, text))
        })?;

        api_response.into_result().map_err(EcloudError::ApiError)
    }

    async fn list_files_internal(
        &self,
        parent_file_id: &str,
        page_size: u32,
        _cursor: Option<String>,
    ) -> Result<EcloudFileListResponse, EcloudError> {
        let path = format!(
            "/v2/listFiles.action?folderId={}&orderBy=lastOpTime&descending=true&showHidden=false&pageNum=1&pageSize={}",
            parent_file_id, page_size
        );

        let response: EcloudFileListResponse = self
            .request(Method::GET, &path)
            .send()
            .await?
            .json()
            .await?;

        Ok(response)
    }

    async fn get_file_meta_by_path(&self, path: &str) -> Result<EcloudFileMeta, EcloudError> {
        let cache = self.path_cache.read().await;
        if let Some((entry, _)) = cache.search(path) {
            return Ok(entry.file_meta.clone());
        }
        Err(EcloudError::NotFound("文件不存在".to_string()))
    }

    async fn get_download_url_by_file_id(
        &self,
        file_id: &str,
    ) -> Result<DownloadableMeta, EcloudError> {
        let path = format!(
            "/v2/getFileDownloadUrl.action?fileId={}&dt=3&flag=1",
            file_id
        );

        let response: DownloadUrlResp = self
            .request(Method::GET, &path)
            .send()
            .await?
            .json()
            .await?;

        Ok(DownloadableMeta {
            download_url: response.download_url,
            size: 0,
            hash: String::new(),
        })
    }

    async fn create_folder_internal(
        &self,
        parent_file_id: &str,
        name: &str,
    ) -> Result<EcloudFileMeta, EcloudError> {
        #[derive(Serialize)]
        struct CreateRequest {
            parentId: String,
            name: String,
        }

        let request = CreateRequest {
            parentId: parent_file_id.to_string(),
            name: name.to_string(),
        };

        let response: CreateFolderResp = self
            .json_request(Method::POST, "/v2/createFolder.action", &request)
            .await?;

        Ok(EcloudFileMeta {
            id: response.id,
            name: response.name,
            file_type: EcloudFileType::Folder,
            size: Some(0),
            last_op_time: None,
            create_date: None,
            md5: None,
        })
    }

    async fn delete_file(&self, file_ids: Vec<String>) -> Result<(), EcloudError> {
        #[derive(Serialize)]
        struct DeleteRequest {
            fileIds: String,
        }

        let request = DeleteRequest {
            fileIds: file_ids.join(","),
        };

        let _: serde_json::Value = self
            .json_request(Method::POST, "/v2/batchDeleteFile.action", &request)
            .await?;

        Ok(())
    }

    async fn rename_file(&self, file_id: &str, new_name: &str) -> Result<(), EcloudError> {
        #[derive(Serialize)]
        struct RenameRequest {
            fileId: String,
            destFileName: String,
        }

        let request = RenameRequest {
            fileId: file_id.to_string(),
            destFileName: new_name.to_string(),
        };

        let _: serde_json::Value = self
            .json_request(Method::GET, "/v2/renameFile.action", &request)
            .await?;

        Ok(())
    }

    async fn copy_file(
        &self,
        file_ids: Vec<String>,
        to_parent_file_id: &str,
    ) -> Result<(), EcloudError> {
        let task_resp = self
            .create_batch_task("COPY", to_parent_file_id, &file_ids)
            .await?;
        self.wait_batch_task(&task_resp.task_id).await?;
        Ok(())
    }

    async fn move_file(
        &self,
        file_ids: Vec<String>,
        to_parent_file_id: &str,
    ) -> Result<(), EcloudError> {
        let task_resp = self
            .create_batch_task("MOVE", to_parent_file_id, &file_ids)
            .await?;
        self.wait_batch_task(&task_resp.task_id).await?;
        Ok(())
    }

    async fn create_batch_task(
        &self,
        task_type: &str,
        to_parent_file_id: &str,
        file_ids: &[String],
    ) -> Result<CreateBatchTaskResp, EcloudError> {
        #[derive(Serialize)]
        struct TaskRequest {
            #[serde(rename = "type")]
            task_type: String,
            targetFolderId: String,
            fileIds: String,
        }

        let request = TaskRequest {
            task_type: task_type.to_string(),
            targetFolderId: to_parent_file_id.to_string(),
            fileIds: file_ids.join(","),
        };

        self.json_request(Method::POST, "/v2/createBatchTask.action", &request)
            .await
    }

    async fn wait_batch_task(&self, task_id: &str) -> Result<(), EcloudError> {
        for _ in 0..30 {
            let path = format!("/v2/checkBatchTask.action?taskId={}", task_id);
            let response: BatchTaskStateResp = self
                .request(Method::GET, &path)
                .send()
                .await?
                .json()
                .await?;

            if response.task_status == 4 {
                return Ok(());
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
        }
        Err(EcloudError::ApiError("任务超时".to_string()))
    }

    async fn init_upload(
        &self,
        parent_file_id: &str,
        file_name: &str,
        file_size: u64,
        _content_hash: &str,
    ) -> Result<InitUploadResp, EcloudError> {
        #[derive(Serialize)]
        struct InitRequest {
            parentFolderId: String,
            fileName: String,
            size: u64,
        }

        let request = InitRequest {
            parentFolderId: parent_file_id.to_string(),
            fileName: file_name.to_string(),
            size: file_size,
        };

        let response: InitUploadResp = self
            .json_request(Method::POST, "/v2/initUploadFile.action", &request)
            .await?;

        Ok(response)
    }

    async fn upload_to_eos<R: tokio::io::AsyncRead + Send + Unpin + 'static>(
        &self,
        upload_url: &str,
        content: R,
        file_size: u64,
    ) -> Result<(), EcloudError> {
        use tokio_util::io::ReaderStream;

        let stream = ReaderStream::new(content);
        let body = reqwest::Body::wrap_stream(stream);

        let response = self
            .client
            .put(upload_url)
            .header("Content-Type", "application/octet-stream")
            .header("Content-Length", file_size.to_string())
            .header("SessionKey", &self.config.session_key)
            .body(body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EcloudError::UploadError(format!(
                "上传失败：{}",
                error_text
            )));
        }

        Ok(())
    }

    async fn commit_upload(
        &self,
        commit_url: &str,
        upload_file_id: &str,
    ) -> Result<(), EcloudError> {
        #[derive(Serialize)]
        struct CommitRequest {
            uploadFileId: String,
        }

        let request = CommitRequest {
            uploadFileId: upload_file_id.to_string(),
        };

        let _: CommitUploadResp = self
            .request(Method::POST, commit_url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        Ok(())
    }

    async fn get_file_id_by_path(&self, path: &str) -> Option<String> {
        let cache = self.path_cache.read().await;
        let matched = cache.search(path)?;
        if matched.1.is_empty() {
            Some(matched.0.file_id().to_string())
        } else {
            None
        }
    }

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

    async fn remove_cache(&self, path: &str) {
        let mut cache = self.path_cache.write().await;
        cache.remove(path);
    }

    async fn build_cache_from_ancestor(
        &self,
        ancestor_file_id: &str,
        target_path: String,
        remainder: &str,
    ) -> Result<(), EcloudError> {
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

        let mut current_file_id = ancestor_file_id.to_string();
        let mut current_path = ancestor_path;
        let mut remaining_parts: Vec<&str> =
            remainder.split('/').filter(|s| !s.is_empty()).collect();

        while !remaining_parts.is_empty() {
            let target_name = remaining_parts.remove(0);
            let cursor = None;
            let mut found = false;

            loop {
                let response = self
                    .list_files_internal(&current_file_id, 100, cursor.clone())
                    .await?;

                for item in response
                    .file_list
                    .files
                    .iter()
                    .chain(response.file_list.folders.iter())
                {
                    if item.name == target_name {
                        current_path = if current_path == "/" {
                            format!("/{}", item.name)
                        } else {
                            format!("{}/{}", current_path, item.name)
                        };
                        current_file_id = item.id.clone();
                        found = true;

                        if item.file_type == EcloudFileType::Folder {
                            self.build_cache_recursive(&item.id, current_path.clone())
                                .await?;
                        }
                        break;
                    }
                }

                if found {
                    break;
                }

                if cursor.is_none() {
                    return Err(EcloudError::NotFound(format!(
                        "Path '{}' not found",
                        target_path
                    )));
                }
            }
        }
        Ok(())
    }

    async fn build_cache_recursive(&self, file_id: &str, path: String) -> Result<(), EcloudError> {
        let mut all_entries = Vec::new();

        loop {
            let response = self.list_files_internal(file_id, 100, None).await?;

            for item in response
                .file_list
                .files
                .iter()
                .chain(response.file_list.folders.iter())
            {
                let item_path = if path == "/" {
                    format!("/{}", item.name)
                } else {
                    format!("{}/{}", path, item.name)
                };

                all_entries.push((item_path.clone(), CacheEntry::new(item.clone())));
                if item.file_type == EcloudFileType::Folder {
                    Box::pin(self.build_cache_recursive(&item.id, item_path)).await?;
                }
            }
            break;
        }

        self.update_cache_batch(all_entries).await;
        Ok(())
    }
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{:x}", now)
}
