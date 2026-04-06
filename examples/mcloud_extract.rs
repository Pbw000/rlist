use rlist::storage::driver::mcloud::client::McloudStorage;
use rlist::storage::model::Storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 注意：请替换为您的实际 token
    let storage = McloudStorage::from_authorization("YOUR_MCLOUD_TOKEN_HERE");

    let folder_path = "/your_file_path_here";
    storage.build_cache("/").await?;
    println!("Cache build success!");

    let result = storage.get_download_meta_by_path(folder_path).await;
    println!("Result:{:?}", result);
    Ok(())
}
