use std::sync::Arc;

use crate::error::{RlistError, StorageError};
use crate::storage::file_meta::Meta;
use crate::storage::model::{FileContent, FileList};
use crate::{Storage, storage::radix_tree::RadixTree};

pub struct FusedStorage<T: Storage> {
    drivers: Vec<Arc<T>>,
    tree: RadixTree<Arc<T>>,
}

impl<T: Storage> FusedStorage<T> {
    pub fn new() -> Self {
        Self {
            drivers: Vec::new(),
            tree: RadixTree::new(),
        }
    }

    pub fn add_driver<U: Into<T>>(&mut self, driver: U, prefix: &str) {
        let driver = Arc::new(driver.into());
        self.drivers.push(driver.clone());
        self.tree.insert(prefix, driver);
    }

    pub fn get_driver<'a>(&'a self, path: &'a str) -> Option<(&'a Arc<T>, &'a str)> {
        self.tree.search(path)
    }

    /// 获取所有已注册的驱动
    pub fn drivers(&self) -> &[Arc<T>] {
        &self.drivers
    }

    pub fn clear(&mut self) {
        self.drivers.clear();
        self.tree.clear();
    }
}

impl<T: Storage> Default for FusedStorage<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Storage + 'static> Storage for FusedStorage<T> {
    type Error = RlistError;

    fn name(&self) -> &str {
        "FusedStorage"
    }

    fn driver_name(&self) -> &str {
        "fused"
    }

    async fn handle_path(&self, path: &str) -> Result<Meta, Self::Error> {
        match self.get_driver(path) {
            Some((driver, remaining_path)) => driver
                .handle_path(remaining_path)
                .await
                .map_err(|e| e.into()),
            None => {
                // 如果没有找到且为根目录，返回根目录元数据
                if path.is_empty() || path == "/" {
                    Ok(Meta::directory("/"))
                } else {
                    Err(RlistError::Storage(StorageError::NotFound))
                }
            }
        }
    }

    async fn list_files(
        &self,
        path: &str,
        page_size: u32,
        cursor: Option<String>,
    ) -> Result<FileList, Self::Error> {
        match self.get_driver(path) {
            Some((driver, remaining_path)) => driver
                .list_files(remaining_path, page_size, cursor)
                .await
                .map_err(|e| e.into()),
            None => {
                // 如果没有找到且为根目录，列出所有驱动作为目录
                if path.is_empty() || path == "/" {
                    let children = self.tree.search_children("/");
                    let items = children
                        .iter()
                        .map(|driver| Meta::directory(driver.0.clone()))
                        .collect::<Vec<_>>();
                    let len = items.len() as u64;
                    Ok(FileList::new(items, len))
                } else {
                    Err(RlistError::Storage(StorageError::NotFound))
                }
            }
        }
    }

    async fn get_meta(&self, path: &str) -> Result<Meta, Self::Error> {
        match self.get_driver(path) {
            Some((driver, remaining_path)) => {
                driver.get_meta(remaining_path).await.map_err(|e| e.into())
            }
            None => Err(RlistError::Storage(StorageError::NotFound)),
        }
    }

    async fn get_download_meta_by_path(
        &self,
        path: &str,
    ) -> Result<crate::storage::file_meta::DownloadableMeta, Self::Error> {
        match self.get_driver(path) {
            Some((driver, remaining_path)) => driver
                .get_download_meta_by_path(remaining_path)
                .await
                .map_err(|e| e.into()),
            None => Err(RlistError::Storage(StorageError::NotFound)),
        }
    }

    async fn build_cache(&self) -> Result<(), Self::Error> {
        let mut joinset = tokio::task::JoinSet::new();
        for driver in &self.drivers {
            let driver = driver.clone();
            joinset.spawn(async move { driver.build_cache().await });
        }
        while let Some(res) = joinset.join_next().await {
            if let Err(e) = res {
                eprintln!("Error building cache: {:?}", e);
                return Err(RlistError::Storage(StorageError::OperationFailed));
            }
        }
        Ok(())
    }

    async fn download_file(&self, path: &str) -> Result<Box<dyn FileContent>, Self::Error> {
        match self.get_driver(path) {
            Some((driver, remaining_path)) => driver
                .download_file(remaining_path)
                .await
                .map_err(|e| e.into()),
            None => Err(RlistError::Storage(StorageError::NotFound)),
        }
    }

    async fn create_folder(&self, path: &str) -> Result<Meta, Self::Error> {
        match self.get_driver(path) {
            Some((driver, remaining_path)) => driver
                .create_folder(remaining_path)
                .await
                .map_err(|e| e.into()),
            None => Err(RlistError::Storage(StorageError::NotFound)),
        }
    }

    async fn delete(&self, path: &str) -> Result<(), Self::Error> {
        match self.get_driver(path) {
            Some((driver, remaining_path)) => {
                driver.delete(remaining_path).await.map_err(|e| e.into())
            }
            None => Err(RlistError::Storage(StorageError::NotFound)),
        }
    }

    async fn rename(&self, old_path: &str, new_name: &str) -> Result<Meta, Self::Error> {
        match self.get_driver(old_path) {
            Some((driver, remaining_path)) => driver
                .rename(remaining_path, new_name)
                .await
                .map_err(|e| e.into()),
            None => Err(RlistError::Storage(StorageError::NotFound)),
        }
    }

    async fn copy(&self, source_path: &str, dest_path: &str) -> Result<Meta, Self::Error> {
        match self.get_driver(source_path) {
            Some((driver, remaining_source)) => driver
                .copy(remaining_source, dest_path)
                .await
                .map_err(|e| e.into()),
            None => Err(RlistError::Storage(StorageError::NotFound)),
        }
    }

    async fn move_(&self, source_path: &str, dest_path: &str) -> Result<Meta, Self::Error> {
        match self.get_driver(source_path) {
            Some((driver, remaining_source)) => driver
                .move_(remaining_source, dest_path)
                .await
                .map_err(|e| e.into()),
            None => Err(RlistError::Storage(StorageError::NotFound)),
        }
    }

    async fn upload_file(&self, path: &str, content: Vec<u8>) -> Result<Meta, Self::Error> {
        match self.get_driver(path) {
            Some((driver, remaining_path)) => driver
                .upload_file(remaining_path, content)
                .await
                .map_err(|e| e.into()),
            None => Err(RlistError::Storage(StorageError::NotFound)),
        }
    }

    fn upload_mode(&self) -> crate::storage::model::UploadMode {
        // 默认返回 Relay 模式
        crate::storage::model::UploadMode::Relay
    }

    async fn get_upload_info(
        &self,
        path: &str,
        size: u64,
    ) -> Result<crate::storage::model::UploadInfo, Self::Error> {
        match self.get_driver(path) {
            Some((driver, remaining_path)) => {
                // 检查驱动是否支持 Direct 模式
                if driver.upload_mode() == crate::storage::model::UploadMode::Direct {
                    driver
                        .get_upload_info(remaining_path, size)
                        .await
                        .map_err(|e| e.into())
                } else {
                    // 不支持 Direct 模式，返回错误
                    Err(RlistError::Storage(StorageError::Unsupported).into())
                }
            }
            None => Err(RlistError::Storage(StorageError::NotFound)),
        }
    }

    fn from_auth_data(_json: &str) -> Result<Self, Self::Error> {
        Err(RlistError::Storage(StorageError::Unsupported))
    }

    fn auth_template(&self) -> String {
        String::from("{}")
    }
}
