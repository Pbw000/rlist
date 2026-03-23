//! 应用状态管理

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::auth::auth::AuthConfig;
use crate::{
    Storage,
    error::RlistError,
    storage::all::{AllDriver, StorageRegistry},
};

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<AppStateInner>,
}

pub struct AppStateInner {
    /// 存储引擎注册表
    pub registry: RwLock<StorageRegistry>,
    /// 存储名称映射
    pub storage_names: RwLock<HashMap<String, String>>,
    /// 管理员密钥
    pub admin_key: String,
    /// 认证配置
    pub auth_config: Arc<AuthConfig>,
    /// 公开存储引擎注册表（用于无需认证的访问）
    pub public_registry: RwLock<StorageRegistry>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(admin_key: String, auth_config: Arc<AuthConfig>) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                registry: RwLock::new(StorageRegistry::new()),
                storage_names: RwLock::new(HashMap::new()),
                admin_key,
                auth_config,
                public_registry: RwLock::new(StorageRegistry::new()),
            }),
        }
    }

    /// 获取存储引擎列表
    pub async fn list_storages(&self) -> Vec<String> {
        self.inner
            .storage_names
            .read()
            .await
            .keys()
            .cloned()
            .collect()
    }

    /// 添加存储引擎
    pub async fn add_storage<T, U>(&self, name: T, prefix: U, driver: impl Into<AllDriver>)
    where
        T: Into<String>,
        U: Into<String>,
    {
        let prefix_str = prefix.into();
        let mut registry = self.inner.registry.write().await;
        registry.add_driver(driver, &prefix_str);
        drop(registry);

        self.inner
            .storage_names
            .write()
            .await
            .insert(name.into(), prefix_str);
    }

    /// 获取存储引擎
    pub async fn get_storage(&self, name: &str) -> Option<String> {
        let names = self.inner.storage_names.read().await;
        names.get(name).cloned()
    }

    /// 移除存储引擎
    pub async fn remove_storage(&self, name: &str) {
        let _prefix = {
            let mut names = self.inner.storage_names.write().await;
            names.remove(name)
        };
    }

    /// 获取所有存储引擎名称和前缀
    pub async fn get_all_storages(&self) -> HashMap<String, String> {
        self.inner.storage_names.read().await.clone()
    }

    /// 获取主注册表的读守卫
    pub async fn get_registry(&self) -> tokio::sync::RwLockReadGuard<'_, StorageRegistry> {
        self.inner.registry.read().await
    }
    pub async fn build_cache(&self, path: &str) -> Result<(), RlistError> {
        self.inner.registry.write().await.build_cache(path).await?;
        self.inner
            .public_registry
            .write()
            .await
            .build_cache(path)
            .await
    }

    /// 完成上传（Direct 模式）
    pub async fn complete_upload(
        &self,
        path: &str,
        upload_id: &str,
        file_id: &str,
        content_hash: &str,
    ) -> Result<Option<crate::storage::model::FileMeta>, RlistError> {
        // path 是绝对路径（包含存储前缀），直接使用
        let registry = self.inner.registry.read().await;
        registry
            .complete_upload(path, upload_id, file_id, content_hash)
            .await
    }

    /// 验证管理员密钥
    pub fn verify_admin_key(&self, key: &str) -> bool {
        key == self.inner.admin_key
    }

    /// 添加公开存储引擎
    pub async fn add_public_storage<T, U>(&self, name: T, prefix: U, driver: impl Into<AllDriver>)
    where
        T: Into<String>,
        U: Into<String>,
    {
        let prefix_str = prefix.into();
        let mut registry = self.inner.public_registry.write().await;
        registry.add_driver(driver, &prefix_str);
        drop(registry);

        self.inner
            .storage_names
            .write()
            .await
            .insert(name.into(), prefix_str);
    }

    /// 获取公开存储引擎注册表
    pub async fn get_public_registry(&self) -> tokio::sync::RwLockReadGuard<'_, StorageRegistry> {
        self.inner.public_registry.read().await
    }
}
