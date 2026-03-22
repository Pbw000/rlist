use crate::error::RlistError;
use crate::storage::FusedStorage;
use crate::{FileMeta, Storage};

use super::driver::local::local::LocalStorage;
use super::driver::mcloud::client::McloudStorage;

pub enum AllDriver {
    Mcloud(McloudStorage),
    LocalStorage(LocalStorage),
}

impl From<McloudStorage> for AllDriver {
    fn from(storage: McloudStorage) -> Self {
        AllDriver::Mcloud(storage)
    }
}

impl From<LocalStorage> for AllDriver {
    fn from(storage: LocalStorage) -> Self {
        AllDriver::LocalStorage(storage)
    }
}
pub type StorageRegistry = FusedStorage<AllDriver>;
impl Storage for AllDriver {
    type Error = RlistError;
    async fn build_cache(&self) -> Result<(), Self::Error> {
        match self {
            AllDriver::Mcloud(driver) => driver.build_cache().await.map_err(|e| e.into()),
            AllDriver::LocalStorage(driver) => driver.build_cache().await.map_err(|e| e.into()),
        }
    }
    fn name(&self) -> &str {
        match self {
            AllDriver::Mcloud(driver) => driver.name(),
            AllDriver::LocalStorage(driver) => driver.name(),
        }
    }

    fn driver_name(&self) -> &str {
        match self {
            AllDriver::Mcloud(driver) => driver.driver_name(),
            AllDriver::LocalStorage(driver) => driver.driver_name(),
        }
    }

    async fn handle_path(&self, path: &str) -> Result<FileMeta, Self::Error> {
        match self {
            AllDriver::Mcloud(driver) => driver.handle_path(path).await.map_err(|e| e.into()),
            AllDriver::LocalStorage(driver) => driver.handle_path(path).await.map_err(|e| e.into()),
        }
    }

    async fn list_files(
        &self,
        path: &str,
        page_size: u32,
        cursor: Option<String>,
    ) -> Result<crate::storage::model::FileList, Self::Error> {
        match self {
            AllDriver::Mcloud(driver) => driver
                .list_files(path, page_size, cursor)
                .await
                .map_err(|e| e.into()),
            AllDriver::LocalStorage(driver) => driver
                .list_files(path, page_size, cursor)
                .await
                .map_err(|e| e.into()),
        }
    }

    async fn get_meta(&self, path: &str) -> Result<FileMeta, Self::Error> {
        match self {
            AllDriver::Mcloud(driver) => driver.get_meta(path).await.map_err(|e| e.into()),
            AllDriver::LocalStorage(driver) => driver.get_meta(path).await.map_err(|e| e.into()),
        }
    }
    async fn get_download_meta_by_path(
        &self,
        path: &str,
    ) -> Result<crate::storage::file_meta::DownloadableMeta, Self::Error> {
        match self {
            AllDriver::Mcloud(driver) => driver
                .get_download_meta_by_path(path)
                .await
                .map_err(|e| e.into()),
            AllDriver::LocalStorage(driver) => driver
                .get_download_meta_by_path(path)
                .await
                .map_err(|e| e.into()),
        }
    }

    async fn download_file(
        &self,
        path: &str,
    ) -> Result<Box<dyn crate::storage::model::FileContent>, Self::Error> {
        match self {
            AllDriver::Mcloud(driver) => driver.download_file(path).await.map_err(|e| e.into()),
            AllDriver::LocalStorage(driver) => {
                driver.download_file(path).await.map_err(|e| e.into())
            }
        }
    }

    async fn create_folder(&self, path: &str) -> Result<FileMeta, Self::Error> {
        match self {
            AllDriver::Mcloud(driver) => {
                // 从路径提取父路径和文件夹名
                let path_trimmed = path.trim_start_matches('/');
                let parts: Vec<&str> = path_trimmed.split('/').collect();
                let (parent_path, name) = if parts.len() >= 2 {
                    let parent = parts[..parts.len() - 1].join("/");
                    let parent = if parent.is_empty() {
                        "/".to_string()
                    } else {
                        format!("/{}", parent)
                    };
                    (parent, parts.last().unwrap_or(&"").to_string())
                } else {
                    ("/".to_string(), parts.last().unwrap_or(&"").to_string())
                };
                let meta = driver
                    .create_folder(&parent_path, &name)
                    .await
                    .map_err(RlistError::from)?;
                Ok(meta.to_meta())
            }
            AllDriver::LocalStorage(driver) => {
                driver.create_folder(path).await.map_err(|e| e.into())
            }
        }
    }

    async fn delete(&self, path: &str) -> Result<(), Self::Error> {
        match self {
            AllDriver::Mcloud(driver) => driver.delete(path).await.map_err(|e| e.into()),
            AllDriver::LocalStorage(driver) => driver.delete(path).await.map_err(|e| e.into()),
        }
    }

    async fn rename(&self, old_path: &str, new_name: &str) -> Result<FileMeta, Self::Error> {
        match self {
            AllDriver::Mcloud(driver) => driver
                .rename(old_path, new_name)
                .await
                .map_err(|e| e.into()),
            AllDriver::LocalStorage(driver) => driver
                .rename(old_path, new_name)
                .await
                .map_err(|e| e.into()),
        }
    }

    async fn copy(&self, source_path: &str, dest_path: &str) -> Result<FileMeta, Self::Error> {
        match self {
            AllDriver::Mcloud(driver) => driver
                .copy(source_path, dest_path)
                .await
                .map_err(|e| e.into()),
            AllDriver::LocalStorage(driver) => driver
                .copy(source_path, dest_path)
                .await
                .map_err(|e| e.into()),
        }
    }

    async fn move_(&self, source_path: &str, dest_path: &str) -> Result<FileMeta, Self::Error> {
        match self {
            AllDriver::Mcloud(driver) => driver
                .move_(source_path, dest_path)
                .await
                .map_err(|e| e.into()),
            AllDriver::LocalStorage(driver) => driver
                .move_(source_path, dest_path)
                .await
                .map_err(|e| e.into()),
        }
    }

    async fn upload_file(&self, path: &str, content: Vec<u8>) -> Result<FileMeta, Self::Error> {
        match self {
            AllDriver::Mcloud(driver) => driver
                .upload_file(path, content)
                .await
                .map_err(|e| e.into()),
            AllDriver::LocalStorage(driver) => driver
                .upload_file(path, content)
                .await
                .map_err(|e| e.into()),
        }
    }

    fn from_auth_data(json: &str) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        match McloudStorage::from_auth_data(json) {
            Ok(mcloud) => Ok(AllDriver::Mcloud(mcloud)),
            Err(_) => {
                let local = LocalStorage::from_auth_data(json)?;
                Ok(AllDriver::LocalStorage(local))
            }
        }
    }

    fn auth_template(&self) -> String
    where
        Self: Sized,
    {
        match self {
            AllDriver::Mcloud(driver) => driver.auth_template(),
            AllDriver::LocalStorage(driver) => driver.auth_template(),
        }
    }
}
