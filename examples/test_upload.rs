use rlist::LocalStorage;
use rlist::storage::all::StorageRegistry;
use rlist::storage::driver::mcloud::client::McloudStorage;
use rlist::storage::model::Storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = McloudStorage::from_authorization(
        "cGM6MTM4ODA2MzAzMDk6ZEpjUHBYbEx8MXxSQ1N8MTc3NjY3NTUyMzc3N3xiT1hRRTZ2eUdKSWYuUnVTd3RHdlY1NWo2N2t5Z2NidTNtbnVuUW5sWTRQRDNicm01aWo2VjB6NmcxWm0wQzBDOG5Qdkl6VWhDMjgzc3NrTjdyOFI2eTJYelQxX3pQenJkdE8zbzNQX2s4V2FKUEFnLnNoemY2MHF0VHJRcU9iWUhaVU4wUlI3T1BkNzYxS2pEUS5fTEdfNGhaYUIuWjJ0T05KakRESEIxQTQt",
    );

    let mut fused = StorageRegistry::new();
    fused.add_driver(storage, "/mcloud");
    fused.build_cache("/").await?;
    println!("Cache build success!");
    let local_storage = LocalStorage::new(r"C:\Users\pang_\Downloads");
    fused.add_driver(local_storage, "/local");
    let result = fused
        .copy_relay(
            "/local/stm32l4xx-hal-master.zip",
            "/mcloud/stm32l4xx-hal-master.zip",
        )
        .await;
    println!("Result:{:?}", result);
    Ok(())
}
