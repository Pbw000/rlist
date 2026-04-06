//! API 配置模块

use serde::{Deserialize, Serialize};

/// API 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// 监听地址
    #[serde(default = "default_addr")]
    pub addr: String,
}

fn default_addr() -> String {
    "0.0.0.0:8080".to_string()
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            addr: default_addr(),
        }
    }
}
