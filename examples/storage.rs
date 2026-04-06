use rlist::LocalStorage;
use rlist::storage::all::StorageRegistry;
use rlist::storage::driver::mcloud::client::McloudStorage;
use rlist::storage::model::Storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 注意：请替换为您的实际 token
    let storage = McloudStorage::from_authorization("YOUR_MCLOUD_TOKEN_HERE");

    let folder_path = "/mcloud/your_file_path_here";
    let mut fused = StorageRegistry::new();
    fused.add_driver(storage, "/mcloud");
    fused.build_cache("/").await?;
    println!("Cache build success!");
    let result = fused.get_download_meta_by_path(folder_path).await;
    println!("Result:{:?}", result);

    // 注意：请替换为您的实际本地路径
    let local_storage = LocalStorage::new("/path/to/your/local/storage");
    fused.add_driver(local_storage, "/local");

    // 复制到已存在的目录
    let result = fused.copy_relay(folder_path, "/local/ddd.apk").await;
    println!("Result:{:?}", result);
    Ok(())
}
