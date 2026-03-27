//! 联通云盘配置

use serde::{Deserialize, Serialize};

/// 联通云盘配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WopanConfig {
    /// 访问令牌
    pub access_token: String,
    /// 刷新令牌
    pub refresh_token: String,
    /// 家庭云 ID（个人云留空）
    #[serde(default)]
    pub family_id: String,
}

impl Default for WopanConfig {
    fn default() -> Self {
        Self {
            access_token: String::new(),
            refresh_token: String::new(),
            family_id: String::new(),
        }
    }
}
