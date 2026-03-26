use crate::error::RlistError;
use crate::storage::FusedStorage;
use crate::storage::fused_storage::partial_storage::PartialStorage;

use super::driver::local::local::LocalStorage;
use super::driver::mcloud::client::McloudStorage;

impl_storage_enum! {
    AllDriver: RlistError,
    drivers: [
        Mcloud: McloudStorage,
        LocalStorage: LocalStorage
    ],
    extension: PartialStorage
}

impl Default for AllDriverConfigMeta {
    fn default() -> Self {
        AllDriverConfigMeta::Mcloud(<McloudStorage as crate::Storage>::ConfigMeta::default())
    }
}

pub type StorageRegistry = FusedStorage<AllDriver>;
