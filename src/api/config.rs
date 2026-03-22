//! API 配置模块

use serde::{Deserialize, Serialize};

/// API 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// 监听地址
    #[serde(default = "default_addr")]
    pub addr: String,
    /// 管理员密钥（用于保护特定路由）
    #[serde(default = "default_admin_key")]
    pub admin_key: String,
}

fn default_addr() -> String {
    "0.0.0.0:8080".to_string()
}

fn default_admin_key() -> String {
    "rlist-admin-key-change-in-production".to_string()
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            addr: default_addr(),
            admin_key: default_admin_key(),
        }
    }
}
