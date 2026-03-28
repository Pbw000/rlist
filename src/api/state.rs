//! 应用状态管理

use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;
use tokio::sync::RwLock;
use tracing::info;

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
    pub challenge: ChallengeTask<300>,
    /// 后台刷新任务通知器
    pub refresh_notifier: tokio::sync::Notify,
    /// 服务启动时间（用于计算 last_visit）
    pub startup_time: Instant,
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

        let state = Self {
            inner: Arc::new(AppStateInner {
                private_registry: RwLock::new(private_registry),
                auth_config,
                public_registry: RwLock::new(public_registry),
                challenge,
                refresh_notifier: tokio::sync::Notify::new(),
                startup_time: Instant::now(),
            }),
        };

        // 启动后台刷新任务
        state.start_refresh_task();

        state
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
    //public registry only
    pub async fn build_cache(&self, path: &str) -> Result<(), RlistError> {
        self.inner
            .private_registry
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
        content_hash: &crate::storage::model::Hash,
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

    fn start_refresh_task(&self) {
        let state = self.clone();
        tokio::spawn(async move {
            let mut cleanup_interval =
                tokio::time::interval(std::time::Duration::from_secs(15 * 60)); // 15 分钟
            loop {
                if let Err(e) = state
                    .inner
                    .private_registry
                    .read()
                    .await
                    .build_cache("/")
                    .await
                {
                    tracing::error!("Failed to build private registry cache: {}", e);
                }
                if let Err(e) = state
                    .inner
                    .public_registry
                    .read()
                    .await
                    .build_cache("/")
                    .await
                {
                    tracing::error!("Failed to build public registry cache: {}", e);
                }
                state.clean_expired_users().await;
                cleanup_interval.tick().await;
                state.inner.refresh_notifier.notified().await;
            }
        });
    }
    pub fn trigger_refresh(&self) {
        self.inner.refresh_notifier.notify_one();
    }

    /// 清理超时用户信息
    async fn clean_expired_users(&self) {
        // 30 分钟未访问视为超时
        let timeout_secs = 30 * 60;
        let now = self.inner.startup_time.elapsed().as_secs();

        let mut users = self.inner.auth_config.users.write().await;
        let mut removed_count = 0;

        users.retain(|_id, user_info| {
            if now - user_info.last_visit_secs > timeout_secs {
                info!(
                    "清理超时用户：{} (last_visit_secs: {})",
                    user_info.user_name, user_info.last_visit_secs
                );
                removed_count += 1;
                false
            } else {
                true
            }
        });

        if removed_count > 0 {
            info!("共清理 {} 个超时用户", removed_count);
        }
    }
}
