//! 用户数据存储模块
//! 使用 SQLite 数据库持久化存储用户名、salt、密码哈希和权限
//! 用户 ID 每次启动时随机生成，存储在内存中

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use rand::{Rng, RngExt};
use sha2::{Digest, Sha512};
use sqlx::SqlitePool;
use std::time::Duration;
use tokio::fs::File;
use tracing::{error, warn};

const SALT_LENGTH: usize = 32;
const MAX_FAIL_COUNT: u32 = 5;
const BAN_DURATION: Duration = Duration::from_secs(600);
const MIN_PASSWORD_LENGTH: usize = 8;
const MAX_USERNAME_LENGTH: usize = 64;

#[derive(Clone, Copy, Debug, Default)]
pub struct UserPermissions {
    pub read: bool,
    pub download: bool,
    pub upload: bool,
    pub delete: bool,
    pub move_obj: bool,
    pub copy: bool,
    pub create_dir: bool,
    pub list: bool,
}

impl UserPermissions {
    /// 转换为位掩码
    pub fn to_bits(&self) -> u8 {
        let mut bits: u8 = 0;
        if self.read {
            bits |= 1 << 0;
        }
        if self.download {
            bits |= 1 << 1;
        }
        if self.upload {
            bits |= 1 << 2;
        }
        if self.delete {
            bits |= 1 << 3;
        }
        if self.move_obj {
            bits |= 1 << 4;
        }
        if self.copy {
            bits |= 1 << 5;
        }
        if self.create_dir {
            bits |= 1 << 6;
        }
        if self.list {
            bits |= 1 << 7;
        }
        bits
    }

    /// 从位掩码创建
    pub fn from_bits(bits: u8) -> Self {
        UserPermissions {
            read: bits & (1 << 0) != 0,
            download: bits & (1 << 1) != 0,
            upload: bits & (1 << 2) != 0,
            delete: bits & (1 << 3) != 0,
            move_obj: bits & (1 << 4) != 0,
            copy: bits & (1 << 5) != 0,
            create_dir: bits & (1 << 6) != 0,
            list: bits & (1 << 7) != 0,
        }
    }

    /// 默认用户权限（只读 + 上传 + 列表 + 创建目录）
    pub fn default_user() -> Self {
        UserPermissions {
            read: true,
            download: true,
            upload: true,
            delete: false,
            move_obj: false,
            copy: false,
            create_dir: true,
            list: true,
        }
    }

    /// 管理员权限（所有权限）
    pub fn admin() -> Self {
        UserPermissions {
            read: true,
            download: true,
            upload: true,
            delete: true,
            move_obj: true,
            copy: true,
            create_dir: true,
            list: true,
        }
    }
}

/// 用户凭证信息（持久化存储：用户名 -> salt + 密码哈希 + 权限）
#[derive(Clone, Debug)]
pub struct UserCredentials {
    pub salt: [u8; SALT_LENGTH],
    pub password_hash: String,
    pub permissions: UserPermissions,
}

/// 用户凭证存储
#[derive(Clone)]
pub struct UserCredentialsStore {
    pool: SqlitePool,
}

impl UserCredentialsStore {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(database_url).parent() {
            let _ = std::fs::create_dir_all(parent);
            File::create_new(database_url).await.ok();
        }
        let pool = SqlitePool::connect(database_url).await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_credentials (
                username TEXT PRIMARY KEY NOT NULL,
                salt BLOB NOT NULL,
                password_hash TEXT NOT NULL,
                permissions INTEGER NOT NULL DEFAULT 255,
                fail_count INTEGER NOT NULL DEFAULT 0,
                ban_exp DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(UserCredentialsStore { pool })
    }

    /// 注册用户凭证
    pub async fn register(
        &self,
        username: String,
        password: String,
        permissions: UserPermissions,
    ) -> Result<(), (StatusCode, String)> {
        // 验证用户名
        if username.is_empty() || username.len() > MAX_USERNAME_LENGTH {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("用户名长度必须在 1-{} 个字符之间", MAX_USERNAME_LENGTH),
            ));
        }

        // 验证密码强度
        if password.len() < MIN_PASSWORD_LENGTH {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("密码长度至少为 {} 个字符", MIN_PASSWORD_LENGTH),
            ));
        }

        // 检查用户名是否已存在
        let exists: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM user_credentials WHERE username = ?")
                .bind(&username)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| {
                    error!("注册时数据库错误：{}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "服务器内部错误".to_string(),
                    )
                })?;

        if exists.is_some() {
            return Err((StatusCode::CONFLICT, "用户名已存在".to_string()));
        }

        // 生成随机 salt
        let mut salt = [0u8; SALT_LENGTH];
        rand::rng().fill_bytes(&mut salt);

        // 生成密码哈希 (salt + username + password)
        let password_hash = hash_password_sha512(&password, &username, &salt);

        // 存储凭证到数据库
        sqlx::query(
            "INSERT INTO user_credentials (username, salt, password_hash, permissions) VALUES (?, ?, ?, ?)",
        )
        .bind(&username)
        .bind(&salt.as_slice())
        .bind(&password_hash)
        .bind(&permissions.to_bits())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("注册用户凭证时数据库错误：{}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "服务器内部错误".to_string(),
            )
        })?;

        Ok(())
    }

    /// 验证用户凭证并返回权限
    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<UserPermissions, (StatusCode, String)> {
        // 从数据库获取用户凭证（包括 fail_count 和 ban_exp）
        let result: Option<(Vec<u8>, String, u8, i32, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT salt, password_hash, permissions, fail_count, ban_exp FROM user_credentials WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("认证时数据库错误：{}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "服务器内部错误".to_string(),
            )
        })?;

        let (salt_bytes, stored_hash, permissions_bits, fail_count, ban_exp) = match result {
            Some((salt, hash, perms, fails, ban)) => (salt, hash, perms, fails, ban),
            None => {
                // 用户不存在时也要消耗时间，防止时序攻击
                let delay = rand::rng().random_range(100..=2000);
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                warn!("用户 '{}' 不存在", username);
                return Err((StatusCode::UNAUTHORIZED, "用户名或密码错误".to_string()));
            }
        };

        // 检查账户是否处于封禁状态
        if let Some(ban_time) = ban_exp {
            if Utc::now() < ban_time {
                warn!("用户 '{}' 处于封禁状态", username);
                return Err((
                    StatusCode::FORBIDDEN,
                    "账户已被封禁，请稍后再试".to_string(),
                ));
            } else {
                // 封禁已过期，重置 fail_count 和 ban_exp
                sqlx::query(
                    "UPDATE user_credentials SET fail_count = 0, ban_exp = NULL WHERE username = ?",
                )
                .bind(username)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    error!("重置封禁状态时数据库错误：{}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "服务器内部错误".to_string(),
                    )
                })?;
            }
        }

        // 验证密码 (使用存储的 salt)
        let salt: [u8; SALT_LENGTH] = salt_bytes.try_into().map_err(|_| {
            error!("用户 '{}' salt 格式错误", username);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "服务器内部错误".to_string(),
            )
        })?;

        let input_hash = hash_password_sha512(password, username, &salt);
        if stored_hash != input_hash {
            let delay = rand::rng().random_range(100..=2000);
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            let new_fail_count = fail_count + 1;
            if new_fail_count >= MAX_FAIL_COUNT as i32 {
                let ban_exp_time = Utc::now() + chrono::Duration::from_std(BAN_DURATION).unwrap();
                sqlx::query(
                    "UPDATE user_credentials SET fail_count = ?, ban_exp = ? WHERE username = ?",
                )
                .bind(new_fail_count)
                .bind(ban_exp_time)
                .bind(username)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    error!("封禁用户 '{}' 时数据库错误：{}", username, e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "服务器内部错误".to_string(),
                    )
                })?;
                warn!("用户 '{}' 密码错误次数过多，已被封禁", username);
                return Err((
                    StatusCode::FORBIDDEN,
                    "密码错误次数过多，账户已被封禁".to_string(),
                ));
            } else {
                // 更新失败计数
                sqlx::query("UPDATE user_credentials SET fail_count = ? WHERE username = ?")
                    .bind(new_fail_count)
                    .bind(username)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| {
                        error!("更新失败计数时数据库错误：{}", e);
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "服务器内部错误".to_string(),
                        )
                    })?;
                warn!("用户 '{}' 密码错误", username);
                return Err((StatusCode::UNAUTHORIZED, "用户名或密码错误".to_string()));
            }
        }

        // 密码正确，重置失败计数
        sqlx::query(
            "UPDATE user_credentials SET fail_count = 0, ban_exp = NULL WHERE username = ?",
        )
        .bind(0)
        .bind(username)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("重置失败计数时数据库错误：{}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "服务器内部错误".to_string(),
            )
        })?;

        Ok(UserPermissions::from_bits(permissions_bits))
    }

    /// 更新用户权限
    pub async fn update_permissions(
        &self,
        username: &str,
        permissions: UserPermissions,
    ) -> Result<(), (StatusCode, String)> {
        sqlx::query("UPDATE user_credentials SET permissions = ? WHERE username = ?")
            .bind(&permissions.to_bits())
            .bind(username)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("更新用户 '{}' 权限时数据库错误：{}", username, e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "服务器内部错误".to_string(),
                )
            })?;
        Ok(())
    }

    /// 检查用户是否存在
    pub async fn exists(&self, username: &str) -> bool {
        let result: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM user_credentials WHERE username = ?")
                .bind(username)
                .fetch_optional(&self.pool)
                .await
                .ok()
                .flatten();
        result.is_some()
    }

    /// 删除用户凭证
    pub async fn remove(&self, username: &str) -> bool {
        let result = sqlx::query("DELETE FROM user_credentials WHERE username = ?")
            .bind(username)
            .execute(&self.pool)
            .await;

        match result {
            Ok(rows) => rows.rows_affected() > 0,
            Err(_) => false,
        }
    }

    pub async fn list_usernames(&self) -> Result<Vec<String>, sqlx::Error> {
        let usernames = sqlx::query_scalar("SELECT username FROM user_credentials")
            .fetch_all(&self.pool)
            .await?;
        Ok(usernames)
    }

    /// 获取用户权限
    pub async fn get_permissions(
        &self,
        username: &str,
    ) -> Result<UserPermissions, (StatusCode, String)> {
        let result: Option<(u8,)> =
            sqlx::query_as("SELECT permissions FROM user_credentials WHERE username = ?")
                .bind(username)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| {
                    error!("获取用户 '{}' 权限时数据库错误：{}", username, e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "服务器内部错误".to_string(),
                    )
                })?;

        match result {
            Some((bits,)) => Ok(UserPermissions::from_bits(bits)),
            None => Err((StatusCode::NOT_FOUND, "用户不存在".to_string())),
        }
    }
}

fn hash_password_sha512(password: &str, username: &str, salt: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.update(salt);
    hasher.update("|");
    hasher.update(username);
    hasher.update("|");
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}
