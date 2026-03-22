//! 应用状态管理

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

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
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(admin_key: String) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                registry: RwLock::new(StorageRegistry::new()),
                storage_names: RwLock::new(HashMap::new()),
                admin_key,
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
    pub async fn add_storage(&self, name: String, prefix: String, driver: AllDriver) {
        let mut registry = self.inner.registry.write().await;
        registry.add_driver(driver, &prefix);
        drop(registry);

        self.inner.storage_names.write().await.insert(name, prefix);
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
    pub async fn build_cache(&self) -> Result<(), RlistError> {
        self.inner.registry.write().await.build_cache().await
    }
    /// 验证管理员密钥
    pub fn verify_admin_key(&self, key: &str) -> bool {
        key == self.inner.admin_key
    }
}
