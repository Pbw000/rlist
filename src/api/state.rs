//! 应用状态管理

use std::sync::Arc;

use serde::Serialize;
use tokio::sync::RwLock;

use crate::auth::auth::AuthConfig;
use crate::auth::challenge::ChallengeTask;
use crate::{
    Storage,
    error::RlistError,
    storage::all::{AllDriver, StorageRegistry},
};
/// 获取存储引擎列表
#[derive(Debug, Serialize)]
pub struct DriverInfo {
    idx: usize,
    driver_name: String,
    name: String,
    path: String,
}
/// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<AppStateInner>,
}

pub struct AppStateInner {
    /// 存储引擎注册表
    pub private_registry: RwLock<StorageRegistry>,
    pub auth_config: Arc<AuthConfig>,
    pub public_registry: RwLock<StorageRegistry>,
    pub challenge: ChallengeTask<4, 300>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(
        auth_config: Arc<AuthConfig>,
        private_registry: StorageRegistry,
        public_registry: StorageRegistry,
    ) -> Self {
        let challenge = ChallengeTask::new();
        challenge.start_rotate(30);
        Self {
            inner: Arc::new(AppStateInner {
                private_registry: RwLock::new(private_registry),
                auth_config,
                public_registry: RwLock::new(public_registry),
                challenge,
            }),
        }
    }
    pub async fn list_public_storages(&self) -> Vec<DriverInfo> {
        self.inner
            .public_registry
            .read()
            .await
            .drivers_with_prefix()
            .iter()
            .enumerate()
            .map(|(idx, o)| DriverInfo {
                idx,
                driver_name: o.driver.driver_name().to_string(),
                name: o.driver.name().to_string(),
                path: o.prefix.clone(),
            })
            .collect()
    }
    pub async fn list_private_storages(&self) -> Vec<DriverInfo> {
        self.inner
            .private_registry
            .read()
            .await
            .drivers_with_prefix()
            .iter()
            .enumerate()
            .map(|(idx, o)| DriverInfo {
                idx,
                driver_name: o.driver.driver_name().to_string(),
                name: o.driver.name().to_string(),
                path: o.prefix.clone(),
            })
            .collect()
    }
    /// 添加存储引擎
    pub async fn add_storage<U>(&self, prefix: U, driver: impl Into<AllDriver>)
    where
        U: AsRef<str>,
    {
        let prefix_str = prefix.as_ref();
        let mut registry = self.inner.private_registry.write().await;
        registry.add_driver(driver, prefix_str);
    }

    /// 移除存储引擎
    pub async fn remove_public_storage(&self, idx: usize) -> Option<String> {
        self.inner
            .public_registry
            .write()
            .await
            .remove_by_idx(idx)
            .map(|d| d.name().to_string())
    }
    pub async fn remove_private_storage(&self, idx: usize) -> Option<String> {
        self.inner
            .private_registry
            .write()
            .await
            .remove_by_idx(idx)
            .map(|d| d.name().to_string())
    }

    /// 获取主注册表的读守卫
    pub async fn get_registry(&self) -> tokio::sync::RwLockReadGuard<'_, StorageRegistry> {
        self.inner.private_registry.read().await
    }
    pub async fn build_cache(&self, path: &str) -> Result<(), RlistError> {
        self.inner
            .private_registry
            .write()
            .await
            .build_cache(path)
            .await?;
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
        let registry = self.inner.private_registry.read().await;
        registry
            .complete_upload(path, upload_id, file_id, content_hash)
            .await
    }

    /// 添加公开存储引擎
    pub async fn add_public_storage<U>(&self, prefix: U, driver: impl Into<AllDriver>)
    where
        U: AsRef<str>,
    {
        let prefix_str = prefix.as_ref();
        let mut registry = self.inner.public_registry.write().await;
        registry.add_driver(driver, prefix_str);
    }

    /// 获取公开存储引擎注册表
    pub async fn get_public_registry(&self) -> tokio::sync::RwLockReadGuard<'_, StorageRegistry> {
        self.inner.public_registry.read().await
    }
}
