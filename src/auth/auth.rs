use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use std::{collections::HashMap, sync::Arc};
use tracing::warn;

use crate::auth::jwt::{generate_token, verify_token};
use crate::auth::user_store::{UserCredentialsStore, UserPermissions};

#[derive(Clone)]
pub struct AuthConfig {
    pub secret_key: Vec<u8>,
    pub users: Arc<RwLock<HashMap<u128, AuthInfo>>>,
    pub username_to_id: Arc<RwLock<HashMap<String, u128>>>,
    pub credentials_store: Arc<UserCredentialsStore>,
}

impl AuthConfig {
    /// 创建新的 AuthConfig（使用现有数据库连接）
    pub async fn new<T: Into<Vec<u8>>>(
        secret_key: T,
        users: Vec<AuthInfo>,
        credentials_store: UserCredentialsStore,
    ) -> Self {
        let mut users_map = HashMap::with_capacity(users.len());
        let mut username_map = HashMap::with_capacity(users.len());
        for user in users {
            let id = rand::random::<u128>();
            username_map.insert(user.user_name.clone(), id);
            users_map.insert(id, user);
        }
        AuthConfig {
            secret_key: secret_key.into(),
            users: Arc::new(RwLock::new(users_map)),
            username_to_id: Arc::new(RwLock::new(username_map)),
            credentials_store: Arc::new(credentials_store),
        }
    }

    /// 创建随机的 AuthConfig（使用现有数据库连接）
    pub async fn random(users: Vec<AuthInfo>, credentials_store: UserCredentialsStore) -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        let mut secret_key = vec![0u8; 128];
        rng.fill_bytes(&mut secret_key);
        let mut users_map = HashMap::with_capacity(users.len());
        let mut username_map = HashMap::with_capacity(users.len());
        for user in users {
            let id = rng.random::<u128>();
            username_map.insert(user.user_name.clone(), id);
            users_map.insert(id, user);
        }
        AuthConfig {
            secret_key,
            users: Arc::new(RwLock::new(users_map)),
            username_to_id: Arc::new(RwLock::new(username_map)),
            credentials_store: Arc::new(credentials_store),
        }
    }

    pub async fn register(
        &self,
        username: impl Into<String>,
        password: impl AsRef<str>,
    ) -> Result<(), (StatusCode, String)> {
        let username = username.into();
        // 检查用户名是否已存在于内存映射中
        {
            let username_guard = self.username_to_id.read().await;
            if username_guard.contains_key(&username) {
                return Err((StatusCode::CONFLICT, "用户名已存在".to_string()));
            }
        }

        // 在凭证存储中注册（使用默认用户权限）
        self.credentials_store
            .register(
                &username,
                password.as_ref(),
                UserPermissions::default_user(),
            )
            .await?;

        // 从数据库获取权限
        let permissions = self.credentials_store.get_permissions(&username).await?;

        // 生成随机用户 ID（每次启动不同）
        let user_id: u128 = rand::random();

        // 创建 AuthInfo 并添加到内存用户列表
        let auth_info = AuthInfo {
            user_name: username.clone(),
            read: permissions.read,
            download: permissions.download,
            upload: permissions.upload,
            delete: permissions.delete,
            move_obj: permissions.move_obj,
            copy: permissions.copy,
            create_dir: permissions.create_dir,
            list: permissions.list,
        };

        {
            let mut users_guard = self.users.write().await;
            let mut username_guard = self.username_to_id.write().await;
            users_guard.insert(user_id, auth_info);
            username_guard.insert(username, user_id);
        }

        Ok(())
    }

    pub async fn login(
        &self,
        username: String,
        password: String,
    ) -> Result<String, (StatusCode, String)> {
        // 验证用户凭证并获取权限
        let permissions = self
            .credentials_store
            .authenticate(&username, &password)
            .await?;

        // 获取或创建用户 ID
        let existing_id = {
            let username_guard = self.username_to_id.read().await;
            username_guard.get(&username).copied()
        };

        let user_id = match existing_id {
            Some(id) => {
                // 更新现有用户的权限
                let auth_info = AuthInfo {
                    user_name: username.clone(),
                    read: permissions.read,
                    download: permissions.download,
                    upload: permissions.upload,
                    delete: permissions.delete,
                    move_obj: permissions.move_obj,
                    copy: permissions.copy,
                    create_dir: permissions.create_dir,
                    list: permissions.list,
                };

                let mut users_guard = self.users.write().await;
                users_guard.insert(id, auth_info);
                id
            }
            None => {
                // 用户已注册但当前实例未加载（可能是重启后），生成新 ID
                let new_id: u128 = rand::random();

                let auth_info = AuthInfo {
                    user_name: username.clone(),
                    read: permissions.read,
                    download: permissions.download,
                    upload: permissions.upload,
                    delete: permissions.delete,
                    move_obj: permissions.move_obj,
                    copy: permissions.copy,
                    create_dir: permissions.create_dir,
                    list: permissions.list,
                };

                let mut users_guard = self.users.write().await;
                let mut username_guard = self.username_to_id.write().await;
                users_guard.insert(new_id, auth_info);
                username_guard.insert(username, new_id);
                new_id
            }
        };

        // 生成 JWT token
        match generate_token(AuthClaim { i: user_id }, &self.secret_key, 12000) {
            Ok(token) => Ok(token),
            Err(_) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Token 生成失败".to_string(),
            )),
        }
    }

    pub async fn remove_user(&self, username: &str) -> bool {
        let ret = self.credentials_store.remove(username).await;
        let user_id = {
            let mut username_guard = self.username_to_id.write().await;
            username_guard.remove(username)
        };

        if let Some(id) = user_id {
            let mut users_guard = self.users.write().await;
            users_guard.remove(&id);
        }
        ret
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthClaim {
    pub i: u128,
}

#[derive(Clone, Debug)]
pub struct AuthInfo {
    pub user_name: String,
    pub read: bool,
    pub download: bool,
    pub upload: bool,
    pub delete: bool,
    pub move_obj: bool,
    pub copy: bool,
    pub create_dir: bool,
    pub list: bool,
}

/// 权限类型枚举
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Permission {
    Read,
    Download,
    Upload,
    Delete,
    Move,
    Copy,
    CreateDir,
    List,
}

impl AuthInfo {
    /// 检查用户是否有指定权限
    pub fn has_permission(&self, permission: &Permission) -> bool {
        match permission {
            Permission::Read => self.read,
            Permission::Download => self.download,
            Permission::Upload => self.upload,
            Permission::Delete => self.delete,
            Permission::Move => self.move_obj,
            Permission::Copy => self.copy,
            Permission::CreateDir => self.create_dir,
            Permission::List => self.list,
        }
    }
}

pub async fn jwt_middleware(
    State(config): State<Arc<AuthConfig>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let (parts, body) = request.into_parts();
    let token = headers
        .get("AUTH-JWT-TOKEN")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            let client_ip = headers
                .get("cf-connecting-ip")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");
            let method = parts.method.as_str();
            let request_path = parts.uri.path();
            warn!(
                target: "security",
                client_ip = %client_ip,
                method = %method,
                request_path = %request_path,
                "Authentication failed: Missing AUTH-JWT-TOKEN header"
            );
            StatusCode::BAD_GATEWAY
        })?;
    let claim = match verify_token::<AuthClaim>(token, &config.secret_key) {
        Ok(claim) => claim,
        Err(err) => {
            let client_ip = headers
                .get("cf-connecting-ip")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");
            let method = parts.method.as_str();
            let request_path = parts.uri.path();
            warn!(
                target: "security",
                client_ip = %client_ip,
                method = %method,
                request_path = %request_path,
                error = %err,
                "Authentication failed: Jwt Verify failed"
            );
            return Err(StatusCode::BAD_GATEWAY);
        }
    };
    if let Some(user_info) = config
        .users
        .read()
        .await
        .get(&claim.i)
        .and_then(|o| Some(o.to_owned()))
    {
        let mut request = Request::from_parts(parts, body);
        request.extensions_mut().insert(user_info);
        Ok(next.run(request).await)
    } else {
        let client_ip = headers
            .get("cf-connecting-ip")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");
        let method = parts.method.as_str();
        let request_path = parts.uri.path();
        warn!(
            target: "security",
            client_ip = %client_ip,
            method = %method,
            request_path = %request_path,
            "Authentication failed: Invalid user ID in JWT claim"
        );
        Err(StatusCode::BAD_GATEWAY)
    }
}
