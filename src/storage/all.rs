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
    extensions: [PartialStorage]
}

pub type StorageRegistry = FusedStorage<AllDriver>;
