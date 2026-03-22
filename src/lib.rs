pub mod crypto;
pub mod error;
pub mod meta;
pub mod storage;

// 导出统一的存储类型和 trait
pub use storage::driver::local::local::LocalStorage;
pub use storage::model::{FileContent, FileList, FileMeta, FileType, Storage, StorageDriver};
