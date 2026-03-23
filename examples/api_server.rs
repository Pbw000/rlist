use rlist::PartialStorage;
use rlist::api::{ApiConfig, AppState, start_server};
use rlist::storage::driver::local::local::LocalStorage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();
    let addr = "localhost:10000".to_owned();
    let admin_key = uuid::Uuid::new_v4().to_string();
    println!("Admin key:{}", admin_key);
    let config = ApiConfig {
        addr: addr.clone(),
        admin_key: admin_key,
    };

    tracing::info!("API 配置：{:?}", config);
    let state = AppState::new(config.admin_key.clone());
    let local_storage = LocalStorage::new(r"C:\Users\pang_\Downloads");
    state
        .add_storage("local_disk", "/local", local_storage)
        .await;
    tracing::info!("已添加本地存储：local");

    use rlist::storage::driver::mcloud::McloudStorage;

    let storage = McloudStorage::from_authorization(
        "cGM6MTM4ODA2MzAzMDk6ZEpjUHBYbEx8MXxSQ1N8MTc3NjY3NTUyMzc3N3xiT1hRRTZ2eUdKSWYuUnVTd3RHdlY1NWo2N2t5Z2NidTNtbnVuUW5sWTRQRDNicm01aWo2VjB6NmcxWm0wQzBDOG5Qdkl6VWhDMjgzc3NrTjdyOFI2eTJYelQxX3pQenJkdE8zbzNQX2s4V2FKUEFnLnNoemY2MHF0VHJRcU9iWUhaVU4wUlI3T1BkNzYxS2pEUS5fTEdfNGhaYUIuWjJ0T05KakRESEIxQTQt",
    );
    let partial_mcloud = PartialStorage::new(storage.clone(), "/From link sharing");
    state.add_storage("mcloud_disk", "/mcloud", storage).await;
    state
        .add_storage("mcloud_disk_part", "/mcloud_part", partial_mcloud)
        .await;

    tracing::info!("已添加移动云盘存储：mcloud");
    state.build_cache("/").await?;
    start_server(state, &addr).await?;

    Ok(())
}
