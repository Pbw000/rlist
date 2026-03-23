use rlist::storage::driver::mcloud::client::McloudStorage;
use rlist::storage::model::Storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = McloudStorage::from_authorization(
        "cGM6MTM4ODA2MzAzMDk6ZEpjUHBYbEx8MXxSQ1N8MTc3NjY3NTUyMzc3N3xiT1hRRTZ2eUdKSWYuUnVTd3RHdlY1NWo2N2t5Z2NidTNtbnVuUW5sWTRQRDNicm01aWo2VjB6NmcxWm0wQzBDOG5Qdkl6VWhDMjgzc3NrTjdyOFI2eTJYelQxX3pQenJkdE8zbzNQX2s4V2FKUEFnLnNoemY2MHF0VHJRcU9iWUhaVU4wUlI3T1BkNzYxS2pEUS5fTEdfNGhaYUIuWjJ0T05KakRESEIxQTQt",
    );

    let folder_path = "/系统桌面_RELEASE-5.39.31.10165-11112057.apk";
    storage.build_cache("/").await?;
    println!("Cache build success!");
    let result = storage.get_download_meta_by_path(folder_path).await;
    println!("Result:{:?}", result);
    Ok(())
}
