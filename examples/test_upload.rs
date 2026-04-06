use rlist::LocalStorage;
use rlist::storage::all::StorageRegistry;
use rlist::storage::driver::mcloud::client::McloudStorage;
use rlist::storage::model::Storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 注意：请替换为您的实际 token
    let storage = McloudStorage::from_authorization("YOUR_MCLOUD_TOKEN_HERE");

    let mut fused = StorageRegistry::new();
    fused.add_driver(storage, "/mcloud");
    fused.build_cache("/").await?;
    println!("Cache build success!");

    // 注意：请替换为您的实际本地路径
    let local_storage = LocalStorage::new("/path/to/your/local/storage");
    fused.add_driver(local_storage, "/local");

    let result = fused
        .copy_relay("/local/your_file.zip", "/mcloud/your_file.zip")
        .await;
    println!("Result:{:?}", result);
    Ok(())
}
