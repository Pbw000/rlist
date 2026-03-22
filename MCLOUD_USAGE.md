# 中国移动云盘（mcloud）使用指南

## ⚠️ 重要提示

中国移动云盘的 API 是**非官方**的，基于浏览器抓包分析实现。由于以下原因，API 可能无法正常工作：

1. **Token 有效期短**：Authorization Token 有效期约 12 小时，过期需要重新获取
2. **API 端点变化**：中国移动云盘正在迁移到新的 API 系统
3. **认证机制复杂**：可能需要额外的签名或加密参数

## 获取授权令牌（重要！）

1. 打开浏览器访问 [中国移动云盘](https://yun.139.com/)
2. **登录你的账号**
3. 按 `F12` 打开开发者工具
4. 切换到 **Network（网络）** 标签
5. 刷新页面，在左侧搜索框输入 `hcy`
6. 找到类似 `https://caiyun.139.com/hcy/file/list` 的请求
7. 点击该请求，在右侧 **Headers** 标签中找到：
   ```
   Authorization: Basic xxxxxxxxxxxxxxx
   ```
8. 复制 `Basic` 后面的内容（**不包含** `Basic` 本身和空格）

**注意**：如果返回 HTML 页面而不是 JSON，说明 Token 已失效，请重新获取！

## 环境变量配置

```bash
# 必需：授权令牌
export MCLOUD_AUTHORIZATION="your_token_here"

# 可选：云盘类型（默认：personal_new）
# 可选值：personal_new, personal, family, group
export MCLOUD_CLOUD_TYPE="personal_new"

# 可选：Cloud ID（家庭云/群组云必需）
export MCLOUD_CLOUD_ID="your_cloud_id"

# 可选：根文件夹 ID（默认：/）
export MCLOUD_ROOT_FOLDER_ID="/"
```

## 运行示例

```bash
# 运行基本示例
cargo run --example mcloud_example

# 或者设置环境变量后运行
MCLOUD_AUTHORIZATION="your_token" cargo run --example mcloud_example
```

## 代码示例

### 基本使用

```rust
use rlist::mcloud::McloudStorage;
use rlist::storage::config::{StorageConfig, AuthConfig, AuthType};
use rlist::storage::Storage;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> rlist::error::Result<()> {
    // 创建配置
    let mut provider_config = HashMap::new();
    provider_config.insert("cloud_type".to_string(), serde_json::json!("personal_new"));
    
    let config = StorageConfig {
        name: "我的移动云盘".to_string(),
        storage_type: rlist::storage::config::StorageType::Custom("mcloud".to_string()),
        root_path: "/".to_string(),
        read_only: false,
        enabled: true,
        order: 0,
        auth: Some(AuthConfig {
            auth_type: AuthType::Token,
            token: Some("your_authorization_token".to_string()),
            ..Default::default()
        }),
        provider_config,
        cache: Some(Default::default()),
    };

    // 创建存储实例
    let storage = McloudStorage::new(config)?;

    // 列出文件
    let entries = storage.list_all("/").await?;
    for entry in entries {
        println!("{} - {:?}", entry.meta.name, entry.meta.file_type);
    }

    Ok(())
}
```

### 使用辅助函数

```rust
use rlist::mcloud::{McloudStorage, create_mcloud_config};
use rlist::mcloud::config::CloudType;

// 创建配置（更简单的方式）
let mcloud_config = create_mcloud_config(
    CloudType::PersonalNew,
    "your_authorization_token",
    None,  // cloud_id（家庭云/群组云需要）
    Some("/"),  // 根文件夹 ID
);
```

### 文件操作

```rust
use rlist::storage::Storage;
use bytes::Bytes;

// 读取文件
let content = storage.read("/path/to/file.txt").await?;

// 上传文件
let content = Bytes::from("Hello, World!");
storage.write("/path/to/file.txt", content).await?;

// 创建目录
storage.mkdir("/new/folder").await?;

// 删除文件
storage.remove("/path/to/file.txt").await?;

// 重命名
storage.rename("/old/name.txt", "/new/name.txt").await?;

// 复制文件
storage.copy("/src/file.txt", "/dst/file.txt").await?;

// 搜索文件
let results = storage.search("/", "keyword").await?;

// 获取下载链接
if let Some(url) = storage.get_download_url("/file.txt").await? {
    println!("下载链接：{}", url);
}
```

### 分页列表

```rust
use rlist::meta::ListOptions;

let options = ListOptions {
    page: 1,           // 页码
    per_page: 50,      // 每页数量
    search: Some("txt".to_string()),  // 搜索关键词
    order_by_name: true,  // 按名称排序
    ascending: true,   // 升序
};

let result = storage.list("/", options).await?;
println!("共 {} 项", result.total);
for entry in result.items {
    println!("{} - {}", entry.meta.name, entry.meta.human_size().unwrap_or_default());
}
```

## 云盘类型说明

| 类型 | 说明 | 必需参数 |
|------|------|----------|
| `personal_new` | 新个人云（推荐） | authorization |
| `personal` | 个人云（旧版） | authorization |
| `family` | 家庭云 | authorization, cloud_id |
| `group` | 共享群 | authorization, cloud_id |

## 注意事项

1. **Token 有效期**：授权令牌有效期约 12 小时，过期需要重新获取
2. **上传限制**：单文件最大 5GB
3. **API 限流**：请避免短时间内大量请求
4. **家庭云/群组云**：必须提供对应的 `cloud_id`

## 故障排除

### 认证失败
```
错误：认证失败：Token 已过期
解决：重新获取 Authorization 令牌
```

### 文件不存在
```
错误：文件不存在
解决：检查路径是否正确，根目录使用 "/" 或 "root"
```

### 权限不足
```
错误：权限不足
解决：检查账号是否有对应目录的访问权限
```

## 支持的 API

| 功能 | 个人云新版 | 个人云旧版 | 家庭云 | 群组云 |
|------|-----------|-----------|--------|--------|
| 列表 | ✅ | ✅ | ✅ | ✅ |
| 下载 | ✅ | ⏳ | ⏳ | ⏳ |
| 上传 | ✅ | ⏳ | ⏳ | ⏳ |
| 创建目录 | ✅ | ✅ | ⏳ | ⏳ |
| 删除 | ✅ | ⏳ | ⏳ | ⏳ |
| 重命名 | ✅ | ⏳ | ❌ | ⏳ |
| 复制 | ✅ | ⏳ | ⏳ | ⏳ |
| 移动 | ⏳ | ⏳ | ⏳ | ⏳ |

✅ 已实现  ⏳ 部分实现  ❌ 不支持
