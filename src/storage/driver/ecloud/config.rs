//! 天翼云盘配置

use serde::{Deserialize, Serialize};

/// 天翼云盘配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcloudConfig {
    /// sessionKey（从浏览器 Network 面板获取）
    pub session_key: String,
    /// sessionSecret（从浏览器 Network 面板获取，用于签名）
    pub session_secret: String,
}

impl Default for EcloudConfig {
    fn default() -> Self {
        Self {
            session_key: "your_session_key".to_string(),
            session_secret: "your_session_secret".to_string(),
        }
    }
}
