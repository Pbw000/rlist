# Rlist

一个基于 Rust 构建的高性能文件管理与存储服务，支持多存储后端统一接入，提供 RESTful API 接口。

## 特性

- 🚀 **高性能**: 基于 Tokio 异步运行时和 Axum Web 框架
- 🔐 **安全认证**: JWT + HMAC 认证，支持细粒度权限管理
- 💾 **多存储后端**: 支持本地存储、移动云盘、联通云盘等多种存储驱动
- 📦 **统一接口**: 抽象统一的 Storage trait，易于扩展新存储后端
- 🎯 **RESTful API**: 提供完整的文件上传、下载、列表、管理等接口
- 🌐 **CORS 支持**: 支持跨域请求
- 📊 **日志记录**: 内置完善的日志系统
- 🔧 **CLI 工具**: 提供命令行管理工具

## 安装

### 从源码编译

```bash
git clone https://github.com/pbw000/rlist.git
cd rlist
cargo build --release
```

编译后的二进制文件位于 `target/release/rlist`

### 下载预编译版本

从 [Releases](https://github.com/pbw000/rlist/releases) 页面下载对应平台的预编译版本。

## 快速开始

### 1. 配置存储

创建 `storage.toml` 配置文件（参考 `storage.toml.example`）：

```toml
[public_registry]
drivers = []

[[private_registry.drivers]]
path = "mcloud"

[private_registry.drivers.config.Mcloud]
token = "YOUR_MCLOUD_TOKEN_HERE"

[[private_registry.drivers]]
path = "local"

[private_registry.drivers.config.LocalStorage]
root_dir = "/path/to/your/datasets"
```

### 2. 启动服务器

```bash
# 使用默认端口 (10000)
./rlist run

# 指定端口
./rlist run --port 8080
```

首次运行时会自动创建 `admin` 账户并生成随机密码，请妥善保存。

### 3. 管理密码

```bash
# 重置密码
./rlist passwd rst -u admin -n newpassword

# 生成随机密码
./rlist passwd random -u admin
```

## API 文档

### 认证相关

| 方法   | 路径                   | 描述           |
| ------ | ---------------------- | -------------- |
| POST   | `/auth/login`          | 用户登录       |
| GET    | `/auth/challenge`      | 获取认证质询   |
| GET    | `/auth/me`             | 获取当前用户信息 |

### 文件操作

| 方法   | 路径                   | 描述           |
| ------ | ---------------------- | -------------- |
| POST   | `/fs/list`             | 列出文件       |
| GET    | `/fs/dir`              | 获取目录信息   |
| GET    | `/fs/get`              | 获取文件信息   |
| GET    | `/fs/download`         | 下载文件       |
| PUT    | `/fs/upload`           | 上传文件       |
| POST   | `/fs/upload-info`      | 获取上传信息   |
| DELETE | `/fs/delete`           | 删除文件       |
| POST   | `/fs/rename`           | 重命名文件     |
| POST   | `/fs/copy`             | 复制文件       |
| POST   | `/fs/move`             | 移动文件       |

### 管理接口

| 方法   | 路径                   | 描述           |
| ------ | ---------------------- | -------------- |
| POST   | `/admin/users`         | 创建用户       |
| GET    | `/admin/users`         | 获取用户列表   |
| DELETE | `/admin/users/:id`     | 删除用户       |
| PUT    | `/admin/users/:id/perm`| 更新用户权限   |

### 公开接口

| 方法   | 路径                   | 描述           |
| ------ | ---------------------- | -------------- |
| GET    | `/health`              | 健康检查       |
| POST   | `/public/list`         | 公开文件列表   |
| GET    | `/public/download`     | 公开文件下载   |

## 存储驱动

### 支持的存储后端

- **LocalStorage**: 本地文件系统
- **Mcloud**: 中国移动云盘
- **Wopan**: 联通云盘

### 添加自定义存储驱动

实现 `Storage` trait 并注册到 `StorageRegistry`：

```rust
use rlist::storage::model::Storage;

pub struct MyStorage {
    // 你的存储实现
}

#[async_trait::async_trait]
impl Storage for MyStorage {
    // 实现必要的方法
}
```

## 配置说明

### storage.toml

| 配置项                      | 类型   | 描述               |
| --------------------------- | ------ | ------------------ |
| `public_registry.drivers`   | Array  | 公开存储驱动列表   |
| `private_registry.drivers`  | Array  | 私有存储驱动列表   |
| `drivers.path`              | String | 驱动挂载路径       |
| `drivers.config`            | Object | 驱动配置           |

## 开发

### 环境要求

- Rust 1.75+ (Edition 2024)
- Cargo

### 运行开发服务器

```bash
cargo run -- run --port 10000
```

### 运行示例

```bash
cargo run --example mcloud
cargo run --example storage
```

### 构建发布版本

```bash
cargo build --release
```

发布版本已启用 LTO、strip 和最优的代码大小优化。

## 项目结构

```
rlist/
├── src/
│   ├── api/           # Web API 路由和中间件
│   ├── auth/          # 认证和授权模块
│   ├── bin/           # CLI 入口
│   ├── error/         # 错误处理
│   ├── storage/       # 存储驱动实现
│   │   ├── driver/    # 各存储后端实现
│   │   ├── model.rs   # 存储模型定义
│   │   └── file_meta.rs # 文件元数据
│   └── utils/         # 工具函数
├── examples/          # 使用示例
├── static_src/        # 静态资源源文件
├── storage.toml       # 存储配置
└── Cargo.toml
```

## 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件
