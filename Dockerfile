# 多阶段构建 - 编译阶段
FROM rust:1.75-alpine AS builder

# 安装必要的构建依赖
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    pkgconfig \
    make \
    git

# 安装 sqlx-cli 用于 SQLite 支持
RUN cargo install sqlx-cli --no-default-features --features sqlite

# 设置工作目录
WORKDIR /app

# 复制 Cargo 配置文件
COPY Cargo.toml Cargo.lock ./

# 创建 src 目录以便复制
COPY src ./src

# 构建 release 版本
RUN cargo build --release --bin rlist

# 复制二进制文件到临时位置
RUN cp /app/target/release/rlist /rlist

# 运行阶段 - 使用最小的 Alpine 镜像
FROM alpine:3.19

# 安装运行时依赖（SQLite 运行时库）
RUN apk add --no-cache \
    libgcc \
    libstdc++ \
    openssl-libs

# 创建应用目录
RUN mkdir -p /app/data

# 从构建阶段复制二进制文件
COPY --from=builder /rlist /app/rlist

# 设置工作目录
WORKDIR /app

# 暴露默认端口
EXPOSE 10000

# 设置 entrypoint
ENTRYPOINT ["/app/rlist", "run"]
