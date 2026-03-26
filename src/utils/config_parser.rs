use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::fs;

use crate::storage::all::AllDriver;
use crate::storage::fused_storage::fused::ConfigMeta;

/// 配置文件保存路径（相对于可执行文件目录）
const CONFIG_SAVE_DEST: &str = "storage.toml";

/// 应用配置
#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct AppCofiguration {
    /// 公共存储注册表（无需认证即可访问）
    pub public_registry: ConfigMeta<AllDriver>,
    /// 私有存储注册表（需要认证访问）
    pub private_registry: ConfigMeta<AllDriver>,
}

impl Default for AppCofiguration {
    fn default() -> Self {
        Self {
            public_registry: ConfigMeta::<AllDriver>::default(),
            private_registry: ConfigMeta::<AllDriver>::default(),
        }
    }
}

pub async fn load_config_from_file() -> Result<AppCofiguration, Box<dyn Error + Send + Sync>> {
    let path = std::env::current_exe()?
        .parent()
        .ok_or("Failed to get executable directory")?
        .join(CONFIG_SAVE_DEST);

    match fs::read_to_string(&path).await {
        Ok(cfg) => match toml::from_str(&cfg) {
            Ok(config) => {
                tracing::info!("Configuration loaded from: {:?}", path);
                Ok(config)
            }
            Err(e) => {
                eprintln!(
                    "Failed to parse configuration, will use default settings.\n Error: {}\n",
                    e
                );
                tracing::warn!("Failed to parse configuration file {:?}: {}", path, e);
                Ok(AppCofiguration::default())
            }
        },
        Err(e) => {
            // 文件不存在时返回默认配置
            if e.kind() == std::io::ErrorKind::NotFound {
                tracing::info!("Configuration file not found, using default settings");
                tracing::info!("Creating default configuration file...");
                let default_config = AppCofiguration::default();
                write_cfg(&default_config).await?;
                Ok(AppCofiguration::default())
            } else {
                tracing::error!("Failed to read configuration file: {}", e);
                Err(Box::new(e))
            }
        }
    }
}

/// 将配置写入文件
pub async fn write_cfg(cfg: &AppCofiguration) -> Result<(), Box<dyn Error + Send + Sync>> {
    let path = std::env::current_exe()?
        .parent()
        .ok_or("Failed to get executable directory")?
        .join(CONFIG_SAVE_DEST);
    let pretty_string = toml::to_string_pretty(&cfg)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&path, pretty_string).await?;
    tracing::info!("Configuration saved to: {:?}", path);
    Ok(())
}

/// 获取配置文件路径
pub fn get_config_path() -> Result<std::path::PathBuf, Box<dyn Error + Send + Sync>> {
    let path = std::env::current_exe()?
        .parent()
        .ok_or("Failed to get executable directory")?
        .join(CONFIG_SAVE_DEST);
    Ok(path)
}
pub fn get_data_base_path() -> Result<std::path::PathBuf, Box<dyn Error + Send + Sync>> {
    let path = std::env::current_exe()?
        .parent()
        .ok_or("Failed to get executable directory")?
        .join("data.db");
    Ok(path)
}
/// 检查配置文件是否存在
pub async fn config_exists() -> bool {
    match get_config_path() {
        Ok(path) => tokio::fs::try_exists(&path).await.unwrap_or(false),
        Err(_) => false,
    }
}
