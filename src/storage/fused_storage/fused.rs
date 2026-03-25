use std::sync::Arc;

use crate::error::{RlistError, StorageError};
use crate::storage::file_meta::Meta;
use crate::storage::model::{FileContent, FileList, UploadInfoParams};
use crate::{Storage, storage::radix_tree::RadixTree};
#[derive(Debug, PartialEq, Eq)]
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
    pub fn add_driver_arc(&mut self, driver: Arc<T>, prefix: &str) {
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

impl<T: Storage + 'static> FusedStorage<T> {
    /// 便捷的复制方法：从源路径复制到目标路径
    pub async fn copy(&self, src_path: &str, dest_path: &str) -> Result<Meta, RlistError> {
        let source_meta = self.gen_copy_meta(src_path).await?;
        self.copy_end_to_end(source_meta, dest_path).await
    }

    /// 便捷的移动方法：从源路径移动到目标路径
    pub async fn move_file(&self, src_path: &str, dest_path: &str) -> Result<Meta, RlistError> {
        let source_meta = self.gen_move_meta(src_path).await?;
        self.move_end_to_end(source_meta, dest_path).await
    }
}

impl<T: Storage + 'static> Storage for FusedStorage<T> {
    type Error = RlistError;
    type End2EndCopyMeta = T::End2EndCopyMeta;
    type End2EndMoveMeta = T::End2EndMoveMeta;

    fn hash(&self) -> u64 {
        use std::hash::Hasher;
        let hasher = std::collections::hash_map::DefaultHasher::new();
        let mut hasher = hasher;
        for driver in &self.drivers {
            hasher.write_u64(driver.hash());
        }
        hasher.finish()
    }
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
                    Err(RlistError::Storage(StorageError::NotFound(
                        "路径未找到".to_string(),
                    )))
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
                if path.is_empty() || path == "/" {
                    let children = self.tree.search_children("/");
                    let items = children
                        .iter()
                        .map(|driver| Meta::directory(driver.0.clone()))
                        .collect::<Vec<_>>();
                    let len = items.len() as u64;
                    Ok(FileList::new(items, len))
                } else {
                    Err(RlistError::Storage(StorageError::NotFound(
                        "路径未找到".to_string(),
                    )))
                }
            }
        }
    }

    async fn get_meta(&self, path: &str) -> Result<Meta, Self::Error> {
        match self.get_driver(path) {
            Some((driver, remaining_path)) => {
                driver.get_meta(remaining_path).await.map_err(|e| e.into())
            }
            None => Err(RlistError::Storage(StorageError::NotFound(
                "路径未找到".to_string(),
            ))),
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
            None => Err(RlistError::Storage(StorageError::NotFound(
                "路径未找到".to_string(),
            ))),
        }
    }

    async fn build_cache(&self, path: &str) -> Result<(), Self::Error> {
        let mut joinset = tokio::task::JoinSet::new();
        let path = path.trim_end_matches('/');
        if path.is_empty() {
            for driver in self.drivers() {
                let driver = driver.clone();
                joinset.spawn(async move { driver.build_cache("/").await });
            }
        } else {
            for (tree_path, driver) in self.tree.iter_path() {
                if path.starts_with(&tree_path) {
                    let driver = driver.clone();
                    let rel_path = path[tree_path.len()..].to_string();
                    joinset.spawn(async move { driver.build_cache(&rel_path).await });
                } else if tree_path.starts_with(path) {
                    let driver = driver.clone();
                    joinset.spawn(async move { driver.build_cache("/").await });
                }
            }
        }

        while let Some(res) = joinset.join_next().await {
            if let Err(e) = res {
                return Err(RlistError::Storage(StorageError::OperationFailed(format!(
                    "构建缓存失败: {}",
                    e
                ))));
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
            None => Err(RlistError::Storage(StorageError::NotFound(
                "路径未找到".to_string(),
            ))),
        }
    }

    async fn create_folder(&self, path: &str) -> Result<Meta, Self::Error> {
        match self.get_driver(path) {
            Some((driver, remaining_path)) => driver
                .create_folder(remaining_path)
                .await
                .map_err(|e| e.into()),
            None => Err(RlistError::Storage(StorageError::NotFound(
                "路径未找到".to_string(),
            ))),
        }
    }

    async fn delete(&self, path: &str) -> Result<(), Self::Error> {
        match self.get_driver(path) {
            Some((driver, remaining_path)) => {
                driver.delete(remaining_path).await.map_err(|e| e.into())
            }
            None => Err(RlistError::Storage(StorageError::NotFound(
                "路径未找到".to_string(),
            ))),
        }
    }

    async fn rename(&self, old_path: &str, new_name: &str) -> Result<Meta, Self::Error> {
        match self.get_driver(old_path) {
            Some((driver, remaining_path)) => driver
                .rename(remaining_path, new_name)
                .await
                .map_err(|e| e.into()),
            None => Err(RlistError::Storage(StorageError::NotFound(
                "路径未找到".to_string(),
            ))),
        }
    }

    async fn copy_end_to_end(
        &self,
        source_meta: Self::End2EndCopyMeta,
        dest_path: &str,
    ) -> Result<Meta, Self::Error> {
        // 获取目标驱动和路径
        let (dest_drive, dest_remaining_path) =
            self.get_driver(dest_path)
                .ok_or(RlistError::Storage(StorageError::NotFound(
                    "Dest storage not found!".to_owned(),
                )))?;

        // 使用目标驱动来执行复制操作
        // 目标驱动会验证 meta 类型是否匹配
        dest_drive
            .copy_end_to_end(source_meta, dest_remaining_path)
            .await
            .map_err(|e| e.into())
    }

    async fn gen_copy_meta(&self, path: &str) -> Result<Self::End2EndCopyMeta, Self::Error> {
        let (driver, remaining_path) =
            self.get_driver(path)
                .ok_or(RlistError::Storage(StorageError::NotFound(
                    "Driver not found".to_string(),
                )))?;
        // 使用源驱动生成 meta
        driver
            .gen_copy_meta(remaining_path)
            .await
            .map_err(|e| e.into())
    }

    async fn move_end_to_end(
        &self,
        source_meta: Self::End2EndMoveMeta,
        dest_path: &str,
    ) -> Result<Meta, Self::Error> {
        // 获取目标驱动和路径
        let (dest_drive, dest_remaining_path) =
            self.get_driver(dest_path)
                .ok_or(RlistError::Storage(StorageError::NotFound(
                    "Dest storage not found!".to_owned(),
                )))?;

        // 使用目标驱动来执行移动操作
        // 目标驱动会验证 meta 类型是否匹配
        dest_drive
            .move_end_to_end(source_meta, dest_remaining_path)
            .await
            .map_err(|e| e.into())
    }

    async fn gen_move_meta(&self, path: &str) -> Result<Self::End2EndMoveMeta, Self::Error> {
        let (driver, remaining_path) =
            self.get_driver(path)
                .ok_or(RlistError::Storage(StorageError::NotFound(
                    "Driver not found".to_string(),
                )))?;
        // 使用源驱动生成 meta
        driver
            .gen_move_meta(remaining_path)
            .await
            .map_err(|e| e.into())
    }

    async fn upload_file<R: tokio::io::AsyncRead + Send + Unpin + 'static>(
        &self,
        path: &str,
        content: R,
        param: UploadInfoParams,
    ) -> Result<Meta, Self::Error> {
        match self.get_driver(path) {
            Some((driver, remaining_path)) => {
                let remaining_param = UploadInfoParams {
                    path: remaining_path.to_string(),
                    size: param.size,
                    hash: param.hash,
                };
                driver
                    .upload_file(remaining_path, content, remaining_param)
                    .await
                    .map_err(|e| e.into())
            }
            None => Err(RlistError::Storage(StorageError::NotFound(
                "路径未找到".to_string(),
            ))),
        }
    }

    async fn get_upload_info(
        &self,
        params: crate::storage::model::UploadInfoParams,
    ) -> Result<crate::storage::model::UploadInfo, Self::Error> {
        match self.get_driver(&params.path) {
            Some((driver, remaining_path)) => driver
                .get_upload_info(crate::storage::model::UploadInfoParams {
                    path: remaining_path.to_string(),
                    size: params.size,
                    hash: params.hash,
                })
                .await
                .map_err(|e| e.into()),
            None => Err(RlistError::Storage(StorageError::NotFound(
                "路径未找到".to_string(),
            ))),
        }
    }

    async fn complete_upload(
        &self,
        path: &str,
        upload_id: &str,
        file_id: &str,
        content_hash: &str,
    ) -> Result<Option<crate::storage::model::FileMeta>, Self::Error> {
        match self.get_driver(path) {
            Some((driver, remaining_path)) => driver
                .complete_upload(remaining_path, upload_id, file_id, content_hash)
                .await
                .map_err(|e| e.into()),
            None => Err(RlistError::Storage(StorageError::NotFound(
                "路径未找到".to_string(),
            ))),
        }
    }

    fn from_auth_data(_json: &str) -> Result<Self, Self::Error> {
        Err(RlistError::Storage(StorageError::Unsupported(
            "不支持从认证数据初始化".to_string(),
        )))
    }

    fn auth_template(&self) -> String {
        String::from("{}")
    }
}
