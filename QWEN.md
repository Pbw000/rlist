# RList 项目上下文

## 项目概述

**RList** 是一个基于 Rust 开发的云存储管理工具，提供统一的存储抽象层（Storage Trait），支持多种存储后端的统一管理。

### 核心技术栈

- **语言**: Rust 2024 Edition
- **Web 框架**: Axum 0.8 + Tower HTTP
- **数据库**: SQLx (SQLite)
- **异步运行时**: Tokio
- **认证**: JWT + Challenge-Response 认证
- **序列化**: Serde, Postcard

### 主要功能

1. **统一存储抽象层** - 通过 `Storage` trait 提供统一的文件操作接口
2. **多存储后端支持**:
   - `LocalStorage` - 本地文件系统
   - `McloudStorage` - 中国移动云盘
   - `PartialStorage` - 只读存储包装器
3. **Web API 服务** - RESTful API + 前端静态资源服务
4. **CLI 工具** - 用户管理和服务器运行
5. **权限系统** - 基于用户的权限控制（管理员/普通用户）

### 项目架构

```
src/
├── api/          # Web API 层 (Axum routes, middleware, state)
├── auth/         # 认证授权 (JWT, Challenge, User store)
├── bin/          # 可执行文件入口 (rlist.rs)
├── error/        # 统一错误处理 (RlistError)
├── storage/      # 存储抽象层核心
│   ├── driver/   # 具体存储驱动实现
│   │   ├── local/    # 本地存储驱动
│   │   └── mcloud/   # 移动云盘驱动
│   ├── fused_storage/ # 存储融合/聚合
│   └── model.rs  # Storage trait 定义
└── utils/        # 工具函数 (CLI, password 等)
```

## 构建与运行

### 环境要求

- Rust 工具链 (支持 edition 2024)
- 可选：`musl-tools` (用于静态链接 Linux 构建)

### 构建命令

```bash
# 开发构建
cargo build

# 发布构建 (优化体积)
cargo build --release

# 运行主程序
cargo run --bin rlist

# 运行示例
cargo run --example mcloud
cargo run --example storage
```

### 运行方式

```bash
# 默认运行 (端口 10000)
cargo run --bin rlist

# 指定端口
cargo run --bin rlist -- run --port 8080

# 用户密码管理
cargo run --bin rlist -- passwd rst --user <username> --new-password <password>
cargo run --bin rlist -- passwd random --user <username>
```

### 服务器启动流程

1. 首次运行会自动提示创建 admin 账户（生成随机密码）
2. 默认添加本地存储 (`/local`) 和移动云盘存储 (`/mcloud`)
3. Web API 启动在 `0.0.0.0:<port>`
4. 前端页面访问：`http://<addr>/public.html`

## 开发规范

### 代码风格

- 使用 `tracing` 进行日志记录
- 错误处理统一使用 `RlistError` 枚举
- 异步代码使用 `tokio` 运行时
- 存储驱动实现 `Storage` trait

### Storage Trait 核心方法

```rust
trait Storage {
    fn name(&self) -> &str;
    fn list_files(&self, path: &str, page_size: u32, cursor: Option<String>) -> Future<Output = Result<FileList>>;
    fn get_meta(&self, path: &str) -> Future<Output = Result<FileMeta>>;
    fn download_file(&self, path: &str) -> Future<Output = Result<Box<dyn FileContent>>>;
    fn upload_file<R: AsyncRead>(&self, path: &str, content: R, param: UploadInfoParams) -> Future<Output = Result<FileMeta>>;
    // ... 更多方法见 src/storage/model.rs
}
```

### 添加新存储驱动

1. 在 `src/storage/driver/` 下创建新目录
2. 实现 `Storage` trait
3. 在 `src/storage/driver/mod.rs` 中导出

### CI/CD

项目使用 GitHub Actions 自动构建和发布：
- Linux (gnu/musl) 和 Windows 多平台构建
- 标签推送时自动创建 Release

## 关键配置

### 发布优化 (Cargo.toml)

```toml
[profile.release]
opt-level = "z"    # 体积优化
lto = true         # 链接时优化
strip = true       # 去除符号表
panic = "abort"    # panic 时直接终止
```

### 数据持久化

- 用户凭证存储在 `data.db` (SQLite)
- 通过 `.gitignore` 排除数据库文件

## 示例代码

### 使用 Storage API

```rust
use rlist::storage::model::Storage;
use rlist::storage::driver::mcloud::McloudStorage;

let storage = McloudStorage::from_authorization("<auth_token>");
let file_list = storage.list_files("root", 50, None).await?;
```

### 添加存储到 AppState

```rust
let state = AppState::new(auth_config);
state.add_storage("/local", LocalStorage::new("/path")).await;
state.add_public_storage("/public", PartialStorage::new(storage, "/path")).await;
```
