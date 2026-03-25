use rlist::PartialStorage;
use rlist::api::{ApiConfig, AppState, start_server};
use rlist::auth::auth::AuthConfig;
use rlist::auth::user_store::{UserCredentialsStore, UserPermissions};
use rlist::storage::driver::local::local::LocalStorage;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();
    let addr = "localhost:10000".to_owned();
    let config = ApiConfig { addr: addr.clone() };

    tracing::info!("API 配置：{:?}", config);
    let credentials_store = UserCredentialsStore::new("users.db").await?;
    tracing::info!("用户凭证数据库已初始化");
    if !credentials_store.exists("admin").await {
        let random_password = generate_random_password();
        if let Err(e) = credentials_store
            .register("admin", &random_password, UserPermissions::admin())
            .await
        {
            tracing::error!("Failed to create admin user: {:?}", e);
            return Err(format!("Failed to create admin user: {:?}", e).into());
        }
        println!("===========================================");
        println!("Admin user created with random password:");
        println!("Username: admin");
        println!("Password: {}", random_password);
        println!("===========================================");
        tracing::info!("Admin user created with random password");
    } else {
        tracing::info!("Admin user already exists");
    }

    // 创建认证配置
    let auth_config = Arc::new(AuthConfig::random(vec![], credentials_store).await);

    let state = AppState::new(auth_config);
    let local_storage = LocalStorage::new(r"C:\Users\pang_\Downloads");
    state
        .add_storage("local_disk", "/local", local_storage)
        .await;
    tracing::info!("已添加本地存储：local");

    use rlist::storage::driver::mcloud::McloudStorage;

    let storage = McloudStorage::from_authorization(
        "cGM6MTM4ODA2MzAzMDk6ZEpjUHBYbEx8MXxSQ1N8MTc3NjY3NTUyMzc3N3xiT1hRRTZ2eUdKSWYuUnVTd3RHdlY1NWo2N2t5Z2NidTNtbnVuUW5sWTRQRDNicm01aWo2VjB6NmcxWm0wQzBDOG5Qdkl6VWhDMjgzc3NrTjdyOFI2eTJYelQxX3pQenJkdE8zbzNQX2s4V2FKUEFnLnNoemY2MHF0VHJRcU9iWUhaVU4wUlI3T1BkNzYxS2pEUS5fTEdfNGhaYUIuWjJ0T05KakRESEIxQTQt",
    );
    let mut partial_mcloud = PartialStorage::new(storage.clone(), "/public/hieulerpi");
    partial_mcloud.read_only(true);
    state.add_storage("mcloud_disk", "/mcloud", storage).await;
    state
        .add_public_storage("mcloud_disk_part", "/hieulerpi", partial_mcloud)
        .await;

    tracing::info!("已添加移动云盘存储：mcloud");
    state.build_cache("/").await?;
    start_server(state, &addr).await?;

    Ok(())
}

fn generate_random_password() -> String {
    use rand::seq::{IteratorRandom, SliceRandom};

    let mut rng = rand::rng();
    let mut password = String::with_capacity(32);
    let uppercase = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let lowercase = "abcdefghijklmnopqrstuvwxyz";
    let digits = "0123456789";
    let special = "!@#$%^&*()_+-=[]{}|;:,.<>?";
    password.push(uppercase.chars().choose(&mut rng).unwrap());
    password.push(lowercase.chars().choose(&mut rng).unwrap());
    password.push(digits.chars().choose(&mut rng).unwrap());
    password.push(special.chars().choose(&mut rng).unwrap());
    let all = format!("{}{}{}{}", uppercase, lowercase, digits, special);
    for _ in 0..28 {
        password.push(all.chars().choose(&mut rng).unwrap());
    }

    // 打乱密码顺序
    let mut chars: Vec<char> = password.chars().collect();
    chars.shuffle(&mut rng);
    chars.into_iter().collect()
}
