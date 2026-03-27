use super::driver::local::local::LocalStorage;
use super::driver::mcloud::client::McloudStorage;
use super::driver::wopan::client::WopanStorage;
use crate::error::RlistError;
use crate::storage::FusedStorage;
use crate::storage::fused_storage::partial_storage::PartialStorage;

impl_storage_enum! {
    AllDriver: RlistError,
    drivers: [
        Mcloud: McloudStorage,
        Wopan: WopanStorage,
        LocalStorage: LocalStorage,
    ],
    extension: PartialStorage
}

impl Default for AllDriverConfigMeta {
    fn default() -> Self {
        AllDriverConfigMeta::Mcloud(<McloudStorage as crate::Storage>::ConfigMeta::default())
    }
}

impl AllDriverConfigMeta {
    /// 获取所有可用的存储驱动列表
    pub fn all_drivers() -> Vec<crate::api::types::StorageDriverInfo> {
        use strum::{EnumMessage, IntoEnumIterator};
        AllDriverConfigMeta::iter()
            .map(|driver| crate::api::types::StorageDriverInfo {
                value: driver.driver_name().to_string(),
                label: driver
                    .get_message()
                    .unwrap_or(&driver.driver_name())
                    .to_string(),
            })
            .collect()
    }
}

pub type StorageRegistry = FusedStorage<AllDriver>;
