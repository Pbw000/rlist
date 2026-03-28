use clap::Parser;
use rlist::Storage;
use rlist::api::{AppState, start_server};
use rlist::auth::auth::AuthConfig;
use rlist::auth::user_store::{UserCredentialsStore, UserPermissions};
use rlist::storage::all::StorageRegistry;
use rlist::utils::cli::{Cli, PasswdSubCommand, RlistSubcommand};
use rlist::utils::config_parser::{get_data_base_path, load_config_from_file};
use rlist::utils::password::generate_random_password;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().init();
    let cli = Cli::parse();
    let credentials_store = UserCredentialsStore::new(get_data_base_path()?).await?;
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
async fn handle_passwd_command(
    credentials_store: &UserCredentialsStore,
    command: PasswdSubCommand,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match command {
        PasswdSubCommand::Rst { user, new_password } => {
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

async fn run_server(
    port: u16,
    credentials_store: UserCredentialsStore,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 首次运行时创建 admin 账户
    if !credentials_store.exists("admin").await {
        create_admin_account(&credentials_store).await?;
    }
    let config = load_config_from_file().await?;
    let auth_config = Arc::new(AuthConfig::random(vec![], credentials_store).await);
    let pub_registry = StorageRegistry::from_auth_data(config.public_registry)?;
    let pri_registry = StorageRegistry::from_auth_data(config.private_registry)?;
    let state = AppState::new(auth_config, pri_registry, pub_registry);

    state.build_cache("/").await?;

    let addr = format!("localhost:{}", port);
    tracing::info!("Starting server on {}", addr);
    start_server(state, &addr).await?;

    Ok(())
}

async fn create_admin_account(
    credentials_store: &UserCredentialsStore,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let random_password = generate_random_password();
    if let Err(e) = credentials_store
        .register("admin", &random_password, UserPermissions::admin(), None)
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
