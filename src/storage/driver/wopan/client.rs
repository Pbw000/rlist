//! 联通云盘 (WoPan) API 客户端 - 类型安全实现

#![allow(dead_code)]

use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::storage::driver::wopan::config::WopanConfig;
use crate::storage::driver::wopan::error::WopanError;
use crate::storage::driver::wopan::types::*;
use crate::storage::file_meta::DownloadableMeta;
use crate::storage::model::{
    FileContent, FileList, FileMeta, Storage, UploadInfo, UploadInfoParams,
};
use crate::storage::radix_tree::RadixTree;
use crate::storage::url_reader::UrlReader;
use reqwest::{Client, RequestBuilder, StatusCode};
use ring::digest::Context;
use ring::digest::SHA256;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

const API_BASE: &str = "https://panservice.mail.wo.cn";
const API_DISPATCHER: &str = "/wohome/dispatcher";

const DEFAULT_CLIENT_ID: &str = "1001000021";
const DEFAULT_CLIENT_SECRET: &str = "XFmi9GS2hzk98jGX";
const DEFAULT_APP_ID: &str = "10000001";
const DEFAULT_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36 Edg/114.0.1823.37";

const SPACE_TYPE_PERSONAL: &str = "0";
const SPACE_TYPE_FAMILY: &str = "1";

const CHANNEL_WOHOME: &str = "wohome";

const KEY_QUERY_ALL_FILES: &str = "QueryAllFiles";
const KEY_GET_DOWNLOAD_URL_V2: &str = "GetDownloadUrlV2";
const KEY_CREATE_DIRECTORY: &str = "CreateDirectory";
const KEY_RENAME_FILE_OR_DIRECTORY: &str = "RenameFileOrDirectory";
const KEY_MOVE_FILE: &str = "MoveFile";
const KEY_COPY_FILE: &str = "CopyFile";
const KEY_DELETE_FILE: &str = "DeleteFile";
const KEY_UPLOAD_2C: &str = "upload2C";
const KEY_FAMILY_USER_CURRENT_ENCODE: &str = "FamilyUserCurrentEncode";
const KEY_GET_ZONE_INFO: &str = "GetZoneInfo";

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub file_meta: WopanFileMeta,
}

impl CacheEntry {
    pub fn new(meta: WopanFileMeta) -> Self {
        Self { file_meta: meta }
    }
    pub fn file_id(&self) -> &str {
        &self.file_meta.id
    }
    pub fn fid(&self) -> &str {
        &self.file_meta.fid
    }
}

#[derive(Debug)]
pub struct WopanStorage {
    config: WopanConfig,
    client: Arc<Client>,
    path_cache: RwLock<RadixTree<CacheEntry>>,
    zone_url: OnceLock<String>,
    default_family_id: OnceLock<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConfigMeta {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(default)]
    pub family_id: String,
}

impl Default for ConfigMeta {
    fn default() -> Self {
        Self {
            access_token: "Your access token here".to_owned(),
            refresh_token: "Your refresh token here".to_owned(),
            family_id: String::new(),
        }
    }
}

impl Clone for WopanStorage {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            client: Arc::clone(&self.client),
            path_cache: RwLock::new(RadixTree::new()),
            zone_url: OnceLock::new(),
            default_family_id: OnceLock::new(),
        }
    }
}

impl WopanStorage {
    pub fn new(config: WopanConfig) -> Self {
        Self {
            config,
            client: Arc::new(Client::builder().user_agent(DEFAULT_UA).build().unwrap()),
            path_cache: RwLock::new(RadixTree::new()),
            zone_url: OnceLock::new(),
            default_family_id: OnceLock::new(),
        }
    }

    pub fn from_tokens(access_token: String, refresh_token: String, family_id: String) -> Self {
        Self::new(WopanConfig {
            access_token,
            refresh_token,
            family_id,
        })
    }

    fn get_space_type(&self) -> &str {
        if self.config.family_id.is_empty() {
            SPACE_TYPE_PERSONAL
        } else {
            SPACE_TYPE_FAMILY
        }
    }

    fn generate_sign(
        key: &str,
        res_time: u128,
        req_seq: u32,
        channel: &str,
        version: &str,
    ) -> String {
        // sign = MD5(key + resTime + reqSeq + channel + version)
        // 参考 alist wopan-sdk-go 实现
        use md5::Digest;
        let mut ctx = md5::Md5::new();
        ctx.update(key.as_bytes());
        ctx.update(res_time.to_string().as_bytes());
        ctx.update(req_seq.to_string().as_bytes());
        ctx.update(channel.as_bytes());
        ctx.update(version.as_bytes());
        hex::encode(ctx.finalize())
    }

    fn encrypt_body(&self, body: &str) -> Result<String, WopanError> {
        // 使用 access_token 的前 16 个字符作为密钥
        let key = if self.config.access_token.len() >= 16 {
            self.config.access_token[..16].as_bytes().to_vec()
        } else {
            return Err(WopanError::CryptoError("Access token too short".into()));
        };
        let iv = *b"wNSOYIB1k1DjY5lA";

        let mut padded = body.as_bytes().to_vec();
        let block_size = 16;
        let padding_len = block_size - (body.len() % block_size);
        for _ in 0..padding_len {
            padded.push(padding_len as u8);
        }

        use aes::cipher::{BlockEncryptMut, KeyIvInit};
        type Aes128Cbc = cbc::Encryptor<aes::Aes128>;

        let mut cipher = Aes128Cbc::new_from_slices(&key, &iv)
            .map_err(|e| WopanError::CryptoError(format!("AES cipher init failed: {}", e)))?;

        let mut encrypted = padded;
        for chunk in encrypted.chunks_exact_mut(16) {
            cipher.encrypt_block_mut(chunk.into());
        }

        use base64::Engine;
        Ok(base64::engine::general_purpose::STANDARD.encode(&encrypted))
    }

    fn decrypt_body(&self, encrypted: &str) -> Result<String, WopanError> {
        // 使用 access_token 的前 16 个字符作为密钥
        let key = if self.config.access_token.len() >= 16 {
            self.config.access_token[..16].as_bytes()
        } else {
            return Err(WopanError::CryptoError("Access token too short".into()));
        };
        let iv = *b"wNSOYIB1k1DjY5lA";

        use aes::cipher::{BlockDecryptMut, KeyIvInit};
        type Aes128Cbc = cbc::Decryptor<aes::Aes128>;

        use base64::Engine;
        let mut decrypted = base64::engine::general_purpose::STANDARD
            .decode(encrypted)
            .map_err(|e| WopanError::CryptoError(format!("Base64 decode failed: {}", e)))?;

        let mut cipher = Aes128Cbc::new_from_slices(&key, &iv)
            .map_err(|e| WopanError::CryptoError(format!("AES cipher init failed: {}", e)))?;

        for chunk in decrypted.chunks_exact_mut(16) {
            cipher.decrypt_block_mut(chunk.into());
        }

        let padding_len = decrypted.last().copied().unwrap_or(0) as usize;
        if padding_len > 0 && padding_len <= 16 {
            decrypted.truncate(decrypted.len() - padding_len);
        }

        String::from_utf8(decrypted)
            .map_err(|e| WopanError::CryptoError(format!("UTF8 decode failed: {}", e)))
    }

    fn build_request<T: Serialize>(
        &self,
        method_key: &str,
        body: T,
    ) -> Result<RequestBuilder, WopanError> {
        use std::borrow::Cow;

        let url = format!("{}{}", API_BASE, API_DISPATCHER);
        // 使用毫秒时间戳（与 JavaScript/Python 一致）
        let res_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        // req_seq 范围：100000-108999（与 alist wopan-sdk-go 一致）
        let req_seq: u32 = rand::random::<u32>() % 9000 + 100000;

        // 根据 alist wopan-sdk-go 实现，sign = MD5(key + resTime + reqSeq + channel + version)
        let sign = Self::generate_sign(method_key, res_time, req_seq, CHANNEL_WOHOME, "");

        // 构建完整的 header
        let header = WopanRequestHeader {
            key: method_key.to_string(),
            res_time,
            req_seq,
            channel: Cow::Borrowed(CHANNEL_WOHOME),
            sign,
            version: Cow::Borrowed(""),
        };

        let request = WopanRequestBody { header, body };
        let request_body = serde_json::to_string(&request)
            .map_err(|e| WopanError::ParseError(format!("Serialize request failed: {}", e)))?;

        Ok(self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("User-Agent", DEFAULT_UA)
            .header("accesstoken", &self.config.access_token)
            .body(request_body))
    }

    async fn send_request<T: Serialize, R: DeserializeOwned>(
        &self,
        method_key: &str,
        body: T,
    ) -> Result<R, WopanError> {
        let body_str = serde_json::to_string(&body)
            .map_err(|e| WopanError::ParseError(format!("Serialize body failed: {}", e)))?;
        let encrypted_body = self.encrypt_body(&body_str)?;

        let encrypted_param = WopanEncryptedParam {
            param: encrypted_body,
            secret: true,
        };

        let request = self.build_request(method_key, encrypted_param)?;
        let response = request.send().await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(WopanError::TokenExpired("HTTP 401 未授权".into()));
        }

        let api_response: WopanDispatcherResponse = response
            .json()
            .await
            .map_err(|e| WopanError::ParseError(format!("读取响应失败：{}", e)))?;

        // dbg!(&api_response);
        let encrypted_data = api_response.into_result()?;
        let decrypted_data = self.decrypt_body(&encrypted_data)?;
        dbg!(&decrypted_data);

        serde_json::from_str(&decrypted_data)
            .map_err(|e| WopanError::ParseError(format!("解析响应数据失败：{}", e)))
    }

    async fn init(&self) -> Result<(), WopanError> {
        let body = FamilyUserCurrentEncodeBody {
            client_id: DEFAULT_CLIENT_ID.to_string(),
        };
        if let Ok(data) = self
            .send_request::<_, WopanFamilyUserData>(KEY_FAMILY_USER_CURRENT_ENCODE, body)
            .await
        {
            let _ = self.default_family_id.set(data.default_home_id.to_string());
        }

        let body = GetZoneInfoBody {
            app_id: DEFAULT_APP_ID.to_string(),
        };
        if let Ok(data) = self
            .send_request::<_, WopanZoneInfoData>(KEY_GET_ZONE_INFO, body)
            .await
        {
            let _ = self.zone_url.set(data.url);
        }
        Ok(())
    }

    async fn get_zone_url(&self) -> Result<&String, WopanError> {
        if let Some(url) = self.zone_url.get() {
            return Ok(url);
        }
        self.init().await?;
        self.zone_url
            .get()
            .ok_or_else(|| WopanError::ApiError("Failed to get zone URL".to_string()))
    }
}

impl PartialEq for WopanStorage {
    fn eq(&self, other: &Self) -> bool {
        self.config.access_token == other.config.access_token
    }
}
impl Eq for WopanStorage {}

impl Storage for WopanStorage {
    type Error = WopanError;
    type End2EndCopyMeta = String;
    type End2EndMoveMeta = String;
    type ConfigMeta = ConfigMeta;

    fn to_auth_data(&self) -> Self::ConfigMeta {
        ConfigMeta {
            access_token: self.config.access_token.clone(),
            refresh_token: self.config.refresh_token.clone(),
            family_id: self.config.family_id.clone(),
        }
    }

    fn hash(&self) -> u64 {
        use std::hash::Hasher;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        h.write(self.config.access_token.as_bytes());
        h.finish()
    }

    fn name(&self) -> &str {
        "联通云盘"
    }
    fn driver_name(&self) -> &str {
        "wopan"
    }

    async fn build_cache(&self, path: &str) -> Result<(), WopanError> {
        if path.is_empty() || path == "/" {
            self.build_cache_recursive("root", "/").await?;
            return Ok(());
        }
        let cache = self.path_cache.read().await;
        if let Some((entry, remainder)) = cache.search(path) {
            if remainder.trim_start_matches('/').is_empty() {
                return Ok(());
            }
            let ancestor_id = entry.file_id().to_owned();
            drop(cache);
            self.build_cache_from_ancestor(&ancestor_id, path, remainder)
                .await?;
        } else {
            drop(cache);
            self.build_cache_recursive("root", "/").await?;
        }
        Ok(())
    }

    async fn handle_path(&self, path: &str) -> Result<FileMeta, WopanError> {
        self.get_meta(path).await
    }

    async fn list_files(
        &self,
        path: &str,
        page_size: u32,
        cursor: Option<String>,
    ) -> Result<FileList, WopanError> {
        {
            let cache = self.path_cache.read().await;
            let children = cache.search_children(path);
            if !children.is_empty() && cursor.is_none() {
                let items: Vec<_> = children
                    .iter()
                    .filter_map(|(_, n)| n.value.as_ref().map(|e| e.file_meta.to_meta()))
                    .collect();
                return Ok(FileList {
                    total: items.len() as u64,
                    items,
                    next_cursor: None,
                });
            }
        }

        let file_id = if path == "/" || path.is_empty() || path == "root" {
            "root".to_string()
        } else {
            self.get_file_id_by_path(path)
                .await
                .ok_or_else(|| WopanError::NotFound("File not found in cache".to_string()))?
        };

        let page_num = cursor.and_then(|c| c.parse::<i32>().ok()).unwrap_or(0);
        let response = self
            .list_files_internal(&file_id, page_num, page_size as i32)
            .await?;

        let parent_path = if path == "root" { "/" } else { path };
        let entries: Vec<(String, CacheEntry)> = response
            .files
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

    async fn get_meta(&self, path: &str) -> Result<FileMeta, WopanError> {
        self.get_file_meta_by_path(path).await.map(|m| m.to_meta())
    }

    async fn get_download_meta_by_path(&self, path: &str) -> Result<DownloadableMeta, WopanError> {
        let file = self.get_file_meta_by_path(path).await?;
        let url = self.get_download_url(&file.fid).await?;
        Ok(DownloadableMeta {
            download_url: url,
            size: file.size.unwrap_or(0),
            hash: None,
        })
    }

    async fn download_file(&self, path: &str) -> Result<Box<dyn FileContent>, WopanError> {
        let meta = self.get_download_meta_by_path(path).await?;
        dbg!(&meta);
        // wopan 下载链接不需要额外认证头，直接返回 URL
        let reader = UrlReader::builder(&meta.download_url)
            .header("Origin", "https://pan.wo.cn")
            .header("Referer", "https://pan.wo.cn")
            .size(meta.size)
            .hash(&meta.hash)
            .build();
        Ok(Box::new(reader) as Box<dyn FileContent>)
    }

    async fn create_folder(&self, path: &str) -> Result<FileMeta, WopanError> {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let (parent_id, folder_name);
        if parts.len() >= 2 {
            let parent_path = parts[..parts.len() - 1].join("/");
            parent_id = self
                .get_file_id_by_path(&parent_path)
                .await
                .unwrap_or_else(|| "root".to_string());
            folder_name = parts.last().unwrap_or(&"").to_string();
        } else {
            parent_id = "root".to_string();
            folder_name = parts.last().unwrap_or(&"").to_string();
        }

        let meta = self
            .create_folder_internal(&parent_id, &folder_name)
            .await?;
        self.update_cache(path, CacheEntry::new(meta.clone())).await;
        Ok(meta.to_meta())
    }

    async fn delete(&self, path: &str) -> Result<(), WopanError> {
        if path == "/" || path.is_empty() {
            return Err(WopanError::ApiError("不能删除根目录".into()));
        }
        let file = self.get_file_meta_by_path(path).await?;
        self.delete_file(vec![file.id.clone()]).await?;
        self.remove_cache(path).await;
        Ok(())
    }

    async fn rename(&self, old_path: &str, new_name: &str) -> Result<(), WopanError> {
        let file = self.get_file_meta_by_path(old_path).await?;
        let parent_path = old_path.rsplit_once('/').map(|(p, _)| p).unwrap_or("/");
        let file_type = if file.file_type == WopanFileType::Folder {
            0
        } else {
            1
        };
        self.rename_file(&file.id, file_type, new_name).await?;
        self.remove_cache(old_path).await;
        self.build_cache(parent_path).await?;
        Ok(())
    }

    async fn copy_end_to_end(
        &self,
        source_meta: String,
        dest_path: &str,
    ) -> Result<(), WopanError> {
        let dest_id = self
            .get_file_id_by_path(dest_path)
            .await
            .unwrap_or_else(|| "root".to_string());
        self.copy_file(vec![source_meta], &dest_id).await
    }

    async fn gen_copy_meta(&self, path: &str) -> Result<String, WopanError> {
        self.get_file_meta_by_path(path).await.map(|f| f.id)
    }

    async fn move_end_to_end(
        &self,
        source_meta: String,
        dest_path: &str,
    ) -> Result<(), WopanError> {
        let dest_id = self
            .get_file_id_by_path(dest_path)
            .await
            .unwrap_or_else(|| "root".to_string());
        self.move_file(vec![source_meta], &dest_id).await
    }

    async fn gen_move_meta(&self, path: &str) -> Result<String, WopanError> {
        self.get_file_meta_by_path(path).await.map(|f| f.id)
    }

    async fn upload_file<R: tokio::io::AsyncRead + Send + Unpin + 'static>(
        &self,
        _path: &str,
        content: R,
        param: UploadInfoParams,
    ) -> Result<FileMeta, WopanError> {
        let path = &param.path;
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let file_name = parts
            .last()
            .ok_or_else(|| WopanError::UploadError("无效的文件路径".into()))?;

        let parent_file_id = if parts.len() >= 2 {
            let parent_path = parts[..parts.len() - 1].join("/");
            self.get_file_id_by_path(&parent_path)
                .await
                .unwrap_or_else(|| "root".to_string())
        } else {
            "root".to_string()
        };
        let hash = param.hash.unwrap_or_else(|| {
            let mut hasher = Context::new(&SHA256);
            let bytes: [u8; 12] = rand::random();
            hasher.update(&bytes);
            hex::encode(hasher.finish())
        });
        let create_result = self
            .create_upload_record(&parent_file_id, file_name, param.size, &hash)
            .await?;
        if !create_result.upload_url.is_empty() {
            self.upload_file_content(&create_result.upload_url, content, param.size)
                .await?;
        }

        let file_path = if path.starts_with('/') {
            path
        } else {
            return Err(WopanError::UploadError("路径必须以 / 开头".into()));
        };
        self.build_cache(file_path).await?;
        self.get_meta(path).await
    }

    async fn get_upload_info(&self, params: UploadInfoParams) -> Result<UploadInfo, WopanError> {
        let parts: Vec<&str> = params.path.trim_start_matches('/').split('/').collect();
        let file_name = parts
            .last()
            .ok_or_else(|| WopanError::UploadError("无效的文件路径".into()))?;

        let parent_file_id = if parts.len() >= 2 {
            let parent_path = parts[..parts.len() - 1].join("/");
            let _ = self.build_cache(&parent_path).await;
            self.get_file_id_by_path(&parent_path)
                .await
                .unwrap_or_else(|| "root".to_string())
        } else {
            "root".to_string()
        };
        let hash = params.hash.unwrap_or_else(|| {
            let mut hasher = Context::new(&SHA256);
            let bytes: [u8; 12] = rand::random();
            hasher.update(&bytes);
            hex::encode(hasher.finish())
        });
        let create_result = self
            .create_upload_record(&parent_file_id, file_name, params.size, &hash)
            .await?;

        if create_result.upload_url.is_empty() {
            self.build_cache(&params.path).await.ok();
            Ok(UploadInfo {
                upload_url: "about:blank".into(),
                method: "POST".into(),
                form_fields: None,
                headers: None,
                complete_url: None,
            })
        } else {
            let mut headers = std::collections::HashMap::new();
            headers.insert("Content-Type".into(), "application/octet-stream".into());
            Ok(UploadInfo {
                upload_url: create_result.upload_url,
                method: "PUT".into(),
                form_fields: None,
                headers: Some(headers),
                complete_url: Some(format!(
                    "/api/fs/upload/complete?path={}&file_id={}&fid={}",
                    params.path, create_result.file_id, create_result.fid
                )),
            })
        }
    }

    async fn complete_upload(
        &self,
        path: &str,
        _upload_id: &str,
        _file_id: &str,
        _fid: &str,
    ) -> Result<Option<FileMeta>, WopanError> {
        let file_path = if path.starts_with('/') {
            path
        } else {
            return Err(WopanError::UploadError("路径必须以 / 开头".into()));
        };
        self.build_cache(file_path).await?;
        Ok(self
            .get_file_meta_by_path(file_path)
            .await
            .ok()
            .map(|v| v.to_meta()))
    }

    fn from_auth_data(data: Self::ConfigMeta) -> Result<Self, WopanError> {
        Ok(Self::from_tokens(
            data.access_token,
            data.refresh_token,
            data.family_id,
        ))
    }

    fn auth_template() -> Self::ConfigMeta {
        ConfigMeta {
            access_token: "Your access token here".to_string(),
            refresh_token: "Your refresh token here".to_string(),
            family_id: String::new(),
        }
    }
}

impl WopanStorage {
    /// 获取 family_id，避免重复的锁操作和克隆
    async fn get_family_id(&self) -> Option<String> {
        if self.config.family_id.is_empty() {
            return None;
        }
        if let Some(id) = self.default_family_id.get() {
            return Some(id.clone());
        }
        // 如果还没有初始化，尝试初始化
        let body = FamilyUserCurrentEncodeBody {
            client_id: DEFAULT_CLIENT_ID.to_string(),
        };
        if let Ok(data) = self
            .send_request::<_, WopanFamilyUserData>(KEY_FAMILY_USER_CURRENT_ENCODE, body)
            .await
        {
            let _ = self.default_family_id.set(data.default_home_id.to_string());
            self.default_family_id.get().cloned()
        } else {
            None
        }
    }

    async fn list_files_internal(
        &self,
        parent_file_id: &str,
        page_num: i32,
        page_size: i32,
    ) -> Result<WopanQueryAllFilesData, WopanError> {
        let space_type = self.get_space_type();
        let family_id = if space_type == SPACE_TYPE_FAMILY {
            self.get_family_id().await
        } else {
            None
        };

        let body = QueryAllFilesBody {
            space_type: space_type.to_string(),
            parent_directory_id: parent_file_id.to_owned(),
            page_num,
            page_size,
            sort_rule: 0,
            client_id: DEFAULT_CLIENT_ID.to_string(),
            family_id,
        };
        self.send_request(KEY_QUERY_ALL_FILES, body).await
    }

    async fn get_file_meta_by_path(&self, path: &str) -> Result<WopanFileMeta, WopanError> {
        let cache = self.path_cache.read().await;
        cache
            .search(path)
            .map(|(e, _)| e.file_meta.clone())
            .ok_or_else(|| WopanError::NotFound("文件不存在".into()))
    }

    async fn get_download_url(&self, fid: &str) -> Result<String, WopanError> {
        let body = GetDownloadUrlV2Body {
            type_: "1".into(),
            fid_list: vec![fid.to_owned()],
            client_id: DEFAULT_CLIENT_ID.to_string(),
        };
        let response: WopanDownloadUrlData =
            self.send_request(KEY_GET_DOWNLOAD_URL_V2, body).await?;
        for url in response.list {
            return Ok(url.download_url);
        }
        Err(WopanError::DownloadError(
            "No download URL in response".into(),
        ))
    }

    async fn create_folder_internal(
        &self,
        parent_file_id: &str,
        name: &str,
    ) -> Result<WopanFileMeta, WopanError> {
        let space_type = self.get_space_type();
        let family_id = if space_type == SPACE_TYPE_FAMILY {
            self.get_family_id().await
        } else {
            None
        };

        let body = CreateDirectoryBody {
            space_type: space_type.to_string(),
            family_id,
            parent_directory_id: parent_file_id.to_owned(),
            directory_name: name.to_owned(),
            client_id: DEFAULT_CLIENT_ID.to_string(),
        };
        let response: WopanCreateDirectoryData =
            self.send_request(KEY_CREATE_DIRECTORY, body).await?;
        Ok(WopanFileMeta {
            family_id: None,
            fid: String::new(),
            creator: None,
            size: Some(0),
            create_time: chrono::Utc::now().format("%Y%m%d%H%M%S").to_string(),
            shooting_time: None,
            id: response.id,
            file_type: WopanFileType::Folder,
            thumb_url: None,
            file_type_str: None,
            name: name.to_owned(),
        })
    }

    async fn delete_file(&self, file_ids: Vec<String>) -> Result<(), WopanError> {
        let space_type = self.get_space_type();
        let body = DeleteFileBody {
            space_type: space_type.to_string(),
            vip_level: "0".into(),
            dir_list: None,
            file_list: file_ids,
            client_id: DEFAULT_CLIENT_ID.to_string(),
        };
        self.send_request::<_, serde_json::Value>(KEY_DELETE_FILE, body)
            .await?;
        Ok(())
    }

    async fn rename_file(
        &self,
        id: &str,
        file_type: i32,
        new_name: &str,
    ) -> Result<(), WopanError> {
        let space_type = self.get_space_type();
        let family_id = if space_type == SPACE_TYPE_FAMILY {
            self.get_family_id().await
        } else {
            None
        };

        let body = RenameFileOrDirectoryBody {
            space_type: space_type.to_string(),
            type_: file_type,
            file_type: "0".into(),
            id: id.to_owned(),
            name: new_name.to_owned(),
            client_id: DEFAULT_CLIENT_ID.to_string(),
            family_id,
        };
        self.send_request::<_, serde_json::Value>(KEY_RENAME_FILE_OR_DIRECTORY, body)
            .await?;
        Ok(())
    }

    async fn copy_file(
        &self,
        file_ids: Vec<String>,
        target_dir_id: &str,
    ) -> Result<(), WopanError> {
        let space_type = self.get_space_type();
        let family_id = if space_type == SPACE_TYPE_FAMILY {
            self.get_family_id().await
        } else {
            None
        };

        let body = CopyFileBody {
            target_dir_id: target_dir_id.to_owned(),
            source_type: space_type.to_string(),
            target_type: space_type.to_string(),
            dir_list: None,
            file_list: file_ids,
            secret: false,
            client_id: DEFAULT_CLIENT_ID.to_string(),
            from_family_id: family_id.clone(),
            family_id,
        };
        self.send_request::<_, serde_json::Value>(KEY_COPY_FILE, body)
            .await?;
        Ok(())
    }

    async fn move_file(
        &self,
        file_ids: Vec<String>,
        target_dir_id: &str,
    ) -> Result<(), WopanError> {
        let space_type = self.get_space_type();
        let family_id = if space_type == SPACE_TYPE_FAMILY {
            self.get_family_id().await
        } else {
            None
        };

        let body = MoveFileBody {
            target_dir_id: target_dir_id.to_owned(),
            source_type: space_type.to_string(),
            target_type: space_type.to_string(),
            dir_list: None,
            file_list: file_ids,
            secret: false,
            client_id: DEFAULT_CLIENT_ID.to_string(),
            from_family_id: family_id.clone(),
            family_id,
        };
        self.send_request::<_, serde_json::Value>(KEY_MOVE_FILE, body)
            .await?;
        Ok(())
    }

    async fn get_file_id_by_path(&self, path: &str) -> Option<String> {
        let cache = self.path_cache.read().await;
        cache.search(path).and_then(|(e, r)| {
            if r.is_empty() {
                Some(e.file_id().to_owned())
            } else {
                None
            }
        })
    }

    async fn update_cache(&self, path: &str, entry: CacheEntry) {
        self.path_cache.write().await.insert(path, entry);
    }

    async fn update_cache_batch(&self, entries: Vec<(String, CacheEntry)>) {
        let mut cache = self.path_cache.write().await;
        for (path, entry) in entries {
            cache.insert(&path, entry);
        }
    }

    async fn remove_cache(&self, path: &str) {
        self.path_cache.write().await.remove(path);
    }

    async fn build_cache_from_ancestor(
        &self,
        ancestor_file_id: &str,
        target_path: &str,
        remainder: &str,
    ) -> Result<(), WopanError> {
        let ancestor_path = if target_path.ends_with(remainder) && !remainder.is_empty() {
            target_path[..target_path.len() - remainder.len()].trim_end_matches('/')
        } else {
            ""
        };
        let ancestor_path = if ancestor_path.is_empty() {
            "/"
        } else {
            ancestor_path
        };

        let mut current_file_id = ancestor_file_id.to_owned();
        let mut current_path = ancestor_path.to_owned();
        let mut remaining_parts: Vec<&str> =
            remainder.split('/').filter(|s| !s.is_empty()).collect();

        while !remaining_parts.is_empty() {
            let target_name = remaining_parts.remove(0);
            let mut cursor = None;
            let mut found = false;

            loop {
                let page_num = cursor.unwrap_or(0);
                let response = self
                    .list_files_internal(&current_file_id, page_num, 100)
                    .await?;
                for item in &response.files {
                    if item.name == target_name {
                        current_path = if current_path == "/" {
                            format!("/{}", item.name)
                        } else {
                            format!("{}/{}", current_path, item.name)
                        };
                        current_file_id = item.id.clone();
                        found = true;
                        if item.file_type == WopanFileType::Folder {
                            Box::pin(self.build_cache_recursive(&item.id, &current_path)).await?;
                        }
                        break;
                    }
                }
                if found {
                    break;
                }
                if response.files.len() >= 100 {
                    cursor = Some(page_num + 1);
                } else {
                    return Err(WopanError::NotFound(format!(
                        "Path '{}' not found",
                        target_path
                    )));
                }
            }
        }
        Ok(())
    }

    async fn build_cache_recursive(&self, file_id: &str, path: &str) -> Result<(), WopanError> {
        let mut all_entries = Vec::new();
        let mut page_num = 0;
        loop {
            let response = self.list_files_internal(file_id, page_num, 100).await?;
            for item in &response.files {
                let item_path = if path == "/" {
                    format!("/{}", item.name)
                } else {
                    format!("{}/{}", path, item.name)
                };
                all_entries.push((item_path.clone(), CacheEntry::new(item.clone())));
                if item.file_type == WopanFileType::Folder {
                    Box::pin(self.build_cache_recursive(&item.id, &item_path)).await?;
                }
            }
            if response.files.len() < 100 {
                break;
            }
            page_num += 1;
        }
        self.update_cache_batch(all_entries).await;
        Ok(())
    }

    async fn create_upload_record(
        &self,
        parent_file_id: &str,
        file_name: &str,
        file_size: u64,
        _content_hash: &str,
    ) -> Result<UploadCreateInfo, WopanError> {
        let space_type = self.get_space_type();
        let zone_url = self.get_zone_url().await?;
        let family_id = if space_type == SPACE_TYPE_FAMILY {
            self.get_family_id().await
        } else {
            None
        };

        let body = Upload2CBody {
            space_type: space_type.to_string(),
            family_id,
            parent_directory_id: parent_file_id.to_owned(),
            file_name: file_name.to_owned(),
            file_size,
            client_id: DEFAULT_CLIENT_ID.to_string(),
            zone_url: zone_url.to_owned(),
        };
        let response: WopanUpload2CResp = self.send_request(KEY_UPLOAD_2C, body).await?;
        Ok(UploadCreateInfo {
            file_id: response.fid.clone(),
            fid: response.fid.clone(),
            upload_url: response.upload_url.unwrap_or_default(),
        })
    }

    async fn upload_file_content<R: tokio::io::AsyncRead + Unpin + Send + 'static>(
        &self,
        upload_url: &str,
        content: R,
        file_size: u64,
    ) -> Result<(), WopanError> {
        use tokio_util::io::ReaderStream;
        let stream = ReaderStream::new(content);
        let body = reqwest::Body::wrap_stream(stream);
        let response = self
            .client
            .put(upload_url)
            .header("Content-Type", "application/octet-stream")
            .header("Content-Length", file_size.to_string())
            .body(body)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(WopanError::UploadError(format!(
                "上传失败：{}",
                response.text().await.unwrap_or_default()
            )));
        }
        Ok(())
    }
}

struct UploadCreateInfo {
    file_id: String,
    fid: String,
    upload_url: String,
}

impl WopanQueryAllFilesData {
    fn to_file_list(&self) -> FileList {
        let items = self.files.iter().map(|f| f.to_meta()).collect();
        FileList::with_cursor(items, self.files.len() as u64, None)
    }
}
