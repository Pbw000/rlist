use crate::Storage;
use crate::storage::model::{FileContent, FileList, FileMeta, UploadInfo, UploadInfoParams};

pub struct PartialStorage<T: Storage> {
    pub inner: T,
    pub prefix_path: String,
    read_only: bool,
}
use std::ops::{Deref, DerefMut};

impl<T: Storage> Deref for PartialStorage<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Storage> DerefMut for PartialStorage<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl<T: Storage> PartialEq for PartialStorage<T> {
    fn eq(&self, other: &Self) -> bool {
        self.prefix_path == other.prefix_path && self.read_only == other.read_only
    }
}

impl<T: Storage> Eq for PartialStorage<T> {}
impl<T: Storage> PartialStorage<T> {
    pub fn new(inner: T, prefix_path: &str) -> Self {
        let prefix_path = prefix_path.trim_matches('/');
        Self {
            inner,
            prefix_path: prefix_path.into(),
            read_only: false,
        }
    }
    pub fn read_only(&mut self, read_only: bool) -> &mut Self {
        self.read_only = read_only;
        self
    }
    pub fn into_inner(self) -> T {
        self.inner
    }

    fn handle_path(&self, path: &str) -> String {
        format!("/{}/{}", self.prefix_path, path.trim_start_matches('/'))
    }
}

impl<T: Storage> Storage for PartialStorage<T> {
    type Error = T::Error;
    type End2EndCopyMeta = T::End2EndCopyMeta;
    type End2EndMoveMeta = T::End2EndMoveMeta;

    fn hash(&self) -> u64 {
        self.inner.hash()
    }
    async fn build_cache(&self, path: &str) -> Result<(), Self::Error> {
        self.inner.build_cache(&self.handle_path(path)).await
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
        if self.read_only {
            return Err(<T as Storage>::Error::from(
                "Storage is read-only".to_string(),
            ));
        }
        self.inner.create_folder(&self.handle_path(path)).await
    }

    async fn delete(&self, path: &str) -> Result<(), Self::Error> {
        if self.read_only {
            return Err(<T as Storage>::Error::from(
                "Storage is read-only".to_string(),
            ));
        }
        self.inner.delete(&self.handle_path(path)).await
    }

    async fn rename(&self, old_path: &str, new_name: &str) -> Result<FileMeta, Self::Error> {
        if self.read_only {
            return Err(<T as Storage>::Error::from(
                "Storage is read-only".to_string(),
            ));
        }
        self.inner
            .rename(&self.handle_path(old_path), new_name)
            .await
    }

    async fn copy_end_to_end(
        &self,
        source_meta: Self::End2EndCopyMeta,
        dest_path: &str,
    ) -> Result<FileMeta, Self::Error> {
        if self.read_only {
            return Err(<T as Storage>::Error::from(
                "Storage is read-only".to_string(),
            ));
        }
        self.inner
            .copy_end_to_end(source_meta, &self.handle_path(dest_path))
            .await
    }

    async fn gen_copy_meta(&self, path: &str) -> Result<Self::End2EndCopyMeta, Self::Error> {
        self.inner.gen_copy_meta(&self.handle_path(path)).await
    }

    async fn move_end_to_end(
        &self,
        source_meta: Self::End2EndMoveMeta,
        dest_path: &str,
    ) -> Result<FileMeta, Self::Error> {
        if self.read_only {
            return Err(<T as Storage>::Error::from(
                "Storage is read-only".to_string(),
            ));
        }
        self.inner
            .move_end_to_end(source_meta, &self.handle_path(dest_path))
            .await
    }

    async fn gen_move_meta(&self, path: &str) -> Result<Self::End2EndMoveMeta, Self::Error> {
        self.inner.gen_move_meta(&self.handle_path(path)).await
    }

    async fn get_upload_info(&self, params: UploadInfoParams) -> Result<UploadInfo, Self::Error> {
        if self.read_only {
            return Err(<T as Storage>::Error::from(
                "Storage is read-only".to_string(),
            ));
        }
        self.inner
            .get_upload_info(UploadInfoParams {
                path: self.handle_path(&params.path),
                size: params.size,
                hash: params.hash,
            })
            .await
    }

    async fn upload_file<R: tokio::io::AsyncRead + Send + Unpin + 'static>(
        &self,
        path: &str,
        content: R,
        param: UploadInfoParams,
    ) -> Result<FileMeta, Self::Error> {
        if self.read_only {
            return Err(<T as Storage>::Error::from(
                "Storage is read-only".to_string(),
            ));
        }
        self.inner
            .upload_file(&self.handle_path(path), content, param)
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
            read_only: false,
        })
    }

    fn auth_template(&self) -> String
    where
        Self: Sized,
    {
        self.inner.auth_template()
    }
}
