use crate::Storage;
use crate::storage::model::{FileContent, FileList, FileMeta, UploadInfo, UploadMode};

pub struct PartialStorage<T: Storage> {
    pub inner: T,
    pub prefix_path: String,
}

impl<T: Storage> PartialStorage<T> {
    pub fn new(inner: T, prefix_path: &str) -> Self {
        let prefix_path = prefix_path.trim_matches('/');
        Self {
            inner,
            prefix_path: prefix_path.into(),
        }
    }
    pub fn into_inner(self) -> T {
        self.inner
    }

    fn handle_path(&self, path: &str) -> String {
        format!("/{}/{}", self.prefix_path, path)
    }
}

impl<T: Storage> Storage for PartialStorage<T> {
    type Error = T::Error;
    async fn build_cache(&self) -> Result<(), Self::Error> {
        self.inner.build_cache().await
    }
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn handle_path(&self, path: &str) -> Result<FileMeta, Self::Error> {
        self.inner.handle_path(&self.handle_path(path)).await
    }

    async fn list_files(
        &self,
        path: &str,
        page_size: u32,
        cursor: Option<String>,
    ) -> Result<FileList, Self::Error> {
        self.inner
            .list_files(&self.handle_path(path), page_size, cursor)
            .await
    }

    async fn get_meta(&self, path: &str) -> Result<FileMeta, Self::Error> {
        self.inner.get_meta(&self.handle_path(path)).await
    }

    async fn get_download_meta_by_path(
        &self,
        path: &str,
    ) -> Result<crate::storage::file_meta::DownloadableMeta, Self::Error> {
        self.inner
            .get_download_meta_by_path(&self.handle_path(path))
            .await
    }

    async fn download_file(
        &self,
        path: &str,
    ) -> Result<Box<dyn FileContent + 'static>, Self::Error> {
        self.inner.download_file(&self.handle_path(path)).await
    }

    async fn create_folder(&self, path: &str) -> Result<FileMeta, Self::Error> {
        self.inner.create_folder(&self.handle_path(path)).await
    }

    async fn delete(&self, path: &str) -> Result<(), Self::Error> {
        self.inner.delete(&self.handle_path(path)).await
    }

    async fn rename(&self, old_path: &str, new_name: &str) -> Result<FileMeta, Self::Error> {
        self.inner
            .rename(&self.handle_path(old_path), new_name)
            .await
    }

    async fn copy(&self, source_path: &str, dest_path: &str) -> Result<FileMeta, Self::Error> {
        self.inner
            .copy(&self.handle_path(source_path), &self.handle_path(dest_path))
            .await
    }

    async fn move_(&self, source_path: &str, dest_path: &str) -> Result<FileMeta, Self::Error> {
        self.inner
            .move_(&self.handle_path(source_path), &self.handle_path(dest_path))
            .await
    }

    fn upload_mode(&self) -> UploadMode {
        self.inner.upload_mode()
    }

    async fn get_upload_info(&self, path: &str, size: u64) -> Result<UploadInfo, Self::Error> {
        self.inner
            .get_upload_info(&self.handle_path(path), size)
            .await
    }

    async fn upload_file(&self, path: &str, content: Vec<u8>) -> Result<FileMeta, Self::Error> {
        self.inner
            .upload_file(&self.handle_path(path), content)
            .await
    }

    fn from_auth_data(json: &str) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let inner = T::from_auth_data(json)?;
        Ok(Self {
            inner,
            prefix_path: String::new(),
        })
    }

    fn auth_template(&self) -> String
    where
        Self: Sized,
    {
        self.inner.auth_template()
    }
}
