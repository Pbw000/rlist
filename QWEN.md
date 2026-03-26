# Rlist 项目概述

## 项目简介

**Rlist** 是一个用 Rust 编写的文件管理工具，提供统一的存储抽象层和 HTTP API 服务。它支持多种存储后端（如本地存储、移动云存储等），并通过统一的接口进行文件操作。

## 技术栈

- **语言**: Rust (Edition 2024)
- **Web 框架**: Axum 0.8 + Axum-extra
- **异步运行时**: Tokio
- **数据库**: SQLite (通过 SQLx)
- **认证**: JWT (jsonwebtoken) + 自定义挑战响应机制
- **配置**: TOML
- **日志**: Tracing + tracing-subscriber
- **CLI**: Clap

## 项目结构

```
rlist/
├── src/
│   ├── bin/
│   │   └── rlist.rs          # 主程序入口，CLI 命令处理
│   ├── api/                   # HTTP API 模块
│   │   ├── admin.rs          # 管理员接口
│   │   ├── config.rs         # API 配置
│   │   ├── middleware.rs     # API 中间件
│   │   ├── public.rs         # 公共接口
│   │   ├── routes.rs         # 路由定义
│   │   ├── state.rs          # 应用状态
│   │   ├── types.rs          # API 类型定义
│   │   └── user.rs           # 用户接口
│   ├── auth/                  # 认证模块
│   │   ├── auth.rs           # 认证配置和 JWT 中间件
│   │   ├── challenge.rs      # 挑战响应认证
│   │   ├── jwt.rs            # JWT 令牌生成/验证
│   │   ├── middleware.rs     # 认证中间件
│   │   ├── mod.rs
│   │   └── user_store.rs     # 用户凭证存储
│   ├── storage/               # 存储抽象层
│   │   ├── driver/           # 存储驱动实现
│   │   │   ├── local/        # 本地存储驱动
│   │   │   └── mcloud/       # 移动云存储驱动
│   │   ├── fused_storage/    # 融合存储（多驱动统一管理）
│   │   ├── all.rs            # 存储注册表
│   │   ├── file_meta.rs      # 文件元数据
│   │   ├── model.rs          # 核心存储 trait 定义
│   │   └── radix_tree.rs     # 基数树（用于路径索引）
│   ├── error/                 # 错误处理
│   ├── utils/                 # 工具模块
│   │   ├── cli.rs            # CLI 命令定义
│   │   ├── config_parser.rs  # 配置文件解析
│   │   └── password.rs       # 密码生成
│   └── lib.rs                 # 库入口
├── examples/                  # 示例代码
├── static/                    # 静态资源（前端页面）
├── storage.toml              # 存储配置文件
├── Cargo.toml                # 项目配置
└── .github/workflows/
    └── release.yml           # CI/CD 发布工作流
```

## 核心功能

### 1. 存储抽象层

定义了统一的 `Storage` trait，支持以下操作：
- 文件列表浏览
- 文件上传/下载
- 文件复制/移动/删除
- 文件夹创建
- 元数据查询

### 2. 支持的存储驱动

- **LocalStorage**: 本地文件系统存储
- **McloudStorage**: 移动云存储

### 3. 认证系统

- JWT 令牌认证
- 用户凭证持久化存储（SQLite）
- 权限控制（读、写、删除、移动、复制等）
- 首次运行自动创建 admin 账户

### 4. CLI 命令

```bash
# 运行服务器（默认端口 10000）
rlist run

# 指定端口运行
rlist run --port 8080

# 重置用户密码
rlist passwd rst -u admin -n newpassword

# 生成随机密码
rlist passwd random -u admin
```

## 构建和运行

### 环境要求

- Rust 1.75+ (Edition 2024)
- SQLite

### 构建

```bash
# 开发构建
cargo build

# 发布构建
cargo build --release
```

### 运行

```bash
# 运行服务器
cargo run --bin rlist -- run

# 或直接使用编译后的二进制
./target/release/rlist run --port 10000
```

### 测试

```bash
cargo test
```

## 配置说明

### storage.toml

配置文件定义了公共和私有存储注册表：

```toml
[public_registry]
drivers = []

[[private_registry.drivers]]
path = "mcloud"

[private_registry.drivers.config.Mcloud]
token = "your-mcloud-token"

[[private_registry.drivers]]
path = "local"

[private_registry.drivers.config.LocalStorage]
root_dir = "/path/to/storage"
```

## 开发约定

### 代码风格

- 使用 Rust 标准格式化工具 `rustfmt`
- 遵循 Rust API 设计指南
- 错误处理使用 `thiserror` 和自定义错误类型
- 异步代码使用 Tokio 运行时

### 模块组织

- 每个模块有清晰的职责划分
- 使用 `mod.rs` 组织子模块
- 公共类型在模块根导出

### 存储驱动开发

实现新的存储驱动需要实现 `Storage` trait：

```rust
pub trait Storage: Send + Sync {
    type Error: Send + Sync + Error + 'static + Into<RlistError> + From<String>;
    // ... 其他关联类型和方法
}
```

## API 端点

服务器启动后：
- **前端页面**: `http://localhost:10000/public.html`
- **API 根路径**: `http://localhost:10000/`
- 认证头：`AUTH-JWT-TOKEN`

## 发布流程

项目配置了 GitHub Actions 自动发布：

1. 推送版本标签（如 `v0.1.0`）
2. 自动构建多平台二进制：
   - Linux GNU (x86_64)
   - Linux musl (x86_64)
   - Windows MSVC (x86_64)
3. 自动创建 GitHub Release 并上传构建产物

## 注意事项

- 首次运行会自动创建 admin 账户并显示随机密码
- 配置文件 `storage.toml` 中的敏感信息（如 token）应妥善保管
- 生产环境建议修改默认端口并配置 HTTPS
