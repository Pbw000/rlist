use rlist::api::{ApiConfig, AppState, start_server};
use rlist::storage::all::AllDriver;
use rlist::storage::driver::local::local::LocalStorage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt().init();

    // 解析命令行参数
    let addr = "localhost:10000".to_owned();
    let admin_key = uuid::Uuid::new_v4().to_string();
    println!("Admin key:{}", admin_key);
    // 创建 API 配置
    let config = ApiConfig {
        addr: addr.clone(),
        admin_key: admin_key,
    };

    tracing::info!("API 配置：{:?}", config);

    // 创建应用状态
    let state = AppState::new(config.admin_key.clone());

    // 添加本地存储示例
    let local_storage = LocalStorage::new("/tmp/rlist_storage");
    state
        .add_storage(
            "local".to_string(),
            "/local".to_string(),
            AllDriver::LocalStorage(local_storage),
        )
        .await;
    tracing::info!("已添加本地存储：local");

    // 如果设置了移动云盘 token，添加移动云盘存储
    use rlist::storage::driver::mcloud::McloudStorage;

    let storage = McloudStorage::from_authorization(
        "cGM6MTM4ODA2MzAzMDk6ZEpjUHBYbEx8MXxSQ1N8MTc3NjY3NTUyMzc3N3xiT1hRRTZ2eUdKSWYuUnVTd3RHdlY1NWo2N2t5Z2NidTNtbnVuUW5sWTRQRDNicm01aWo2VjB6NmcxWm0wQzBDOG5Qdkl6VWhDMjgzc3NrTjdyOFI2eTJYelQxX3pQenJkdE8zbzNQX2s4V2FKUEFnLnNoemY2MHF0VHJRcU9iWUhaVU4wUlI3T1BkNzYxS2pEUS5fTEdfNGhaYUIuWjJ0T05KakRESEIxQTQt",
    );
    state
        .add_storage(
            "mcloud".to_string(),
            "/mcloud".to_string(),
            AllDriver::Mcloud(storage),
        )
        .await;

    tracing::info!("已添加移动云盘存储：mcloud");
    state.build_cache().await?;
    start_server(state, &addr).await?;

    Ok(())
}
