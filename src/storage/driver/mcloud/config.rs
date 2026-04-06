//! 中国移动云盘配置

use serde::{Deserialize, Serialize};

/// 中国移动云盘配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McloudConfig {
    pub authorization: String,
}
