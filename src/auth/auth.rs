use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio::sync::RwLock;

use std::{collections::HashMap, sync::Arc};
use tracing::warn;

use crate::auth::jwt::{generate_token, verify_token};
use crate::auth::user_store::{UserCredentialsStore, UserPermissions};

#[derive(Clone)]
pub struct AuthConfig<const SECRET_KEY_LEN: usize = 128, const USERS_SALT_LEN: usize = 128> {
    pub secret_key: [u8; SECRET_KEY_LEN],
    pub users: Arc<RwLock<HashMap<u64, AuthInfo>>>,
    pub users_salt: [u8; USERS_SALT_LEN],
    pub credentials_store: Arc<UserCredentialsStore>,
    /// 服务启动时间（用于计算 last_visit）
    pub startup_time: Instant,
}

impl<const SECRET_KEY_LEN: usize, const USERS_SALT_LEN: usize>
    AuthConfig<SECRET_KEY_LEN, USERS_SALT_LEN>
{
    pub async fn new<T: Into<Vec<u8>>>(
        secret_key: [u8; SECRET_KEY_LEN],
        users: Vec<AuthInfo>,
        credentials_store: UserCredentialsStore,
    ) -> Self {
        let mut users_map = HashMap::with_capacity(users.len());
        let mut rng = rand::rng();
        let mut salt = [0u8; USERS_SALT_LEN];
        rng.fill_bytes(&mut salt);
        for user in users {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            user.user_name.hash(&mut hasher);
            salt.hash(&mut hasher);
            let id = hasher.finish();
            users_map.insert(id, user);
        }
        AuthConfig {
            secret_key,
            users: Arc::new(RwLock::new(users_map)),
            users_salt: salt,
            credentials_store: Arc::new(credentials_store),
            startup_time: Instant::now(),
        }
    }
    pub fn username_to_id(&self, user_name: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        user_name.hash(&mut hasher);
        self.users_salt.hash(&mut hasher);
        hasher.finish()
    }
    pub async fn random(users: Vec<AuthInfo>, credentials_store: UserCredentialsStore) -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        let mut secret_key = [0u8; SECRET_KEY_LEN];
        rng.fill_bytes(&mut secret_key);
        let mut users_map = HashMap::with_capacity(users.len());
        let mut salt = [0u8; USERS_SALT_LEN];
        rng.fill_bytes(&mut salt);
        for user in users {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            user.user_name.hash(&mut hasher);
            salt.hash(&mut hasher);
            let id = hasher.finish();
            users_map.insert(id, user);
        }
        AuthConfig {
            secret_key,
            users: Arc::new(RwLock::new(users_map)),
            users_salt: salt,
            credentials_store: Arc::new(credentials_store),
            startup_time: Instant::now(),
        }
    }

    pub async fn register(
        &self,
        username: impl Into<String>,
        password: impl AsRef<str>,
    ) -> Result<(), (StatusCode, String)> {
        let username = username.into();
        let uid = self.username_to_id(&username);
        {
            if self.users.read().await.contains_key(&uid) {
                return Err((StatusCode::CONFLICT, "用户名已存在".to_string()));
            }
        }

        // 在凭证存储中注册（使用默认用户权限）
        self.credentials_store
            .register(
                &username,
                password.as_ref(),
                UserPermissions::default_user(),
                None, // 默认无根目录限制
            )
            .await?;

        let user_config = self.credentials_store.get_user_config(&username).await?;

        let last_visit_secs = self.startup_time.elapsed().as_secs();

        let auth_info = AuthInfo {
            user_name: username,
            permission: user_config.permissions,
            root_dir: user_config.root_dir,
            last_visit_secs,
        };

        {
            let mut users_guard = self.users.write().await;
            users_guard.insert(uid, auth_info);
        }

        Ok(())
    }

    pub async fn login(
        &self,
        username: String,
        password: String,
    ) -> Result<String, (StatusCode, String)> {
        // 验证用户凭证并获取用户配置（包含权限和根目录）
        let user_config = self
            .credentials_store
            .authenticate(&username, &password)
            .await?;

        // 获取用户 ID
        let user_id = self.username_to_id(&username);

        // 获取相对于启动时间的秒数
        let last_visit_secs = self.startup_time.elapsed().as_secs();

        // 更新或创建用户信息
        let auth_info = AuthInfo {
            user_name: username,
            permission: user_config.permissions,
            root_dir: user_config.root_dir,
            last_visit_secs,
        };

        {
            let mut users_guard = self.users.write().await;
            users_guard.insert(user_id, auth_info);
        }

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
        let user_id = self.username_to_id(username);
        let mut users_guard = self.users.write().await;
        users_guard.remove(&user_id);
        ret
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthClaim {
    pub i: u64,
}

#[derive(Debug)]
pub struct AuthInfo {
    pub user_name: String,
    pub permission: UserPermissions,
    pub root_dir: Option<String>,
    /// 相对于服务启动时间的秒数（通过 AppStateInner::startup_time 计算）
    pub last_visit_secs: u64,
}

impl Clone for AuthInfo {
    fn clone(&self) -> Self {
        Self {
            user_name: self.user_name.clone(),
            permission: self.permission,
            root_dir: self.root_dir.clone(),
            last_visit_secs: self.last_visit_secs,
        }
    }
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
            Permission::Read => self.permission.read,
            Permission::Download => self.permission.download,
            Permission::Upload => self.permission.upload,
            Permission::Delete => self.permission.delete,
            Permission::Move => self.permission.move_obj,
            Permission::Copy => self.permission.copy,
            Permission::CreateDir => self.permission.create_dir,
            Permission::List => self.permission.list,
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
    // 获取用户信息
    let user_info = {
        let users_guard = config.users.read().await;
        users_guard.get(&claim.i).cloned()
    };

    if let Some(mut user_info) = user_info {
        // 更新 last_visit_secs 为当前经过的时间
        user_info.last_visit_secs = config.startup_time.elapsed().as_secs();

        // 同步回用户列表
        {
            let mut users_guard = config.users.write().await;
            if let Some(entry) = users_guard.get_mut(&claim.i) {
                entry.last_visit_secs = user_info.last_visit_secs;
            }
        }

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
