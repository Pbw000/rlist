use clap::Parser;
use rlist::PartialStorage;
use rlist::api::{AppState, start_server};
use rlist::auth::auth::AuthConfig;
use rlist::auth::user_store::{UserCredentialsStore, UserPermissions};
use rlist::storage::driver::local::local::LocalStorage;
use rlist::utils::cli::{Cli, PasswdSubCommand, RlistSubcommand};
use rlist::utils::password::generate_random_password;
use std::sync::Arc;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();
    let cli = Cli::parse();
    let credentials_store = UserCredentialsStore::new("data.db").await?;
    match cli.command {
        Some(RlistSubcommand::Passwd(command)) => {
            handle_passwd_command(&credentials_store, command).await?;
        }
        Some(RlistSubcommand::Run { port }) => {
            run_server(port, credentials_store).await?;
        }
        None => {
            run_server(10000, credentials_store).await?;
        }
    }

    Ok(())
}

/// 处理密码管理命令
async fn handle_passwd_command(
    credentials_store: &UserCredentialsStore,
    command: PasswdSubCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        PasswdSubCommand::Rst { user, new_password } => {
            // 重置用户密码
            if !credentials_store.exists(&user).await {
                return Err(format!("User '{}' does not exist", user).into());
            }

            credentials_store
                .update_password(&user, &new_password)
                .await
                .map_err(|e| format!("Failed to update password: {:?}", e))?;

            println!("===========================================");
            println!("Password reset successfully:");
            println!("Username: {}", user);
            println!("===========================================");
            tracing::info!("Password reset for user: {}", user);
        }
        PasswdSubCommand::Random { user } => {
            // 为用户生成随机密码
            if !credentials_store.exists(&user).await {
                return Err(format!("User '{}' does not exist", user).into());
            }

            let random_password = generate_random_password();
            credentials_store
                .update_password(&user, &random_password)
                .await
                .map_err(|e| format!("Failed to update password: {:?}", e.1))?;

            println!("===========================================");
            println!("Random password generated:");
            println!("Username: {}", user);
            println!("Password: {}", random_password);
            println!("===========================================");
            tracing::info!("Random password generated for user: {}", user);
        }
    }

    Ok(())
}

/// 运行服务器
async fn run_server(
    port: u16,
    credentials_store: UserCredentialsStore,
) -> Result<(), Box<dyn std::error::Error>> {
    if !credentials_store.exists("admin").await {
        println!("Admin account not found.\n Would you like to create an admin account? (y/n)");
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input");
        if input.trim().to_lowercase() == "y" {
            create_admin_account(&credentials_store).await?;
        } else {
            return Err("Admin account creation was skipped".into());
        }
    }

    // 创建认证配置
    let auth_config = Arc::new(AuthConfig::random(vec![], credentials_store).await);

    let state = AppState::new(auth_config);
    let local_storage = LocalStorage::new(r"C:\Users\pang_\Downloads");
    state.add_storage("/local", local_storage).await;
    tracing::info!("已添加本地存储：local");

    use rlist::storage::driver::mcloud::McloudStorage;

    let storage = McloudStorage::from_authorization(
        "cGM6MTM4ODA2MzAzMDk6ZEpjUHBYbEx8MXxSQ1N8MTc3NjY3NTUyMzc3N3xiT1hRRTZ2eUdKSWYuUnVTd3RHdlY1NWo2N2t5Z2NidTNtbnVuUW5sWTRQRDNicm01aWo2VjB6NmcxWm0wQzBDOG5Qdkl6VWhDMjgzc3NrTjdyOFI2eTJYelQxX3pQenJkdE8zbzNQX2s4V2FKUEFnLnNoemY2MHF0VHJRcU9iWUhaVU4wUlI3T1BkNzYxS2pEUS5fTEdfNGhaYUIuWjJ0T05KakRESEIxQTQt",
    );
    let mut partial_mcloud = PartialStorage::new(storage.clone(), "/public/hieulerpi");
    partial_mcloud.read_only(true);
    state.add_storage("/mcloud", storage).await;
    state.add_public_storage("/hieulerpi", partial_mcloud).await;

    tracing::info!("已添加移动云盘存储：mcloud");
    state.build_cache("/").await?;

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Starting server on {}", addr);
    start_server(state, &addr).await?;

    Ok(())
}

async fn create_admin_account(
    credentials_store: &UserCredentialsStore,
) -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}
