//! API 路由定义

use axum::{
    Router, middleware,
    routing::{delete, get, post, put},
};

use crate::api::handlers;
use crate::api::middleware::admin_auth_middleware;
use crate::api::state::AppState;
use crate::auth::auth::Permission;
use crate::auth::middleware::{AuthMiddlewareState, auth_permission_middleware};

/// 创建 API 路由（不绑定状态）
pub fn create_routes(state: AppState) -> Router<AppState> {
    // 公开认证路由（无需认证）
    let public_auth_routes = Router::new()
        .route("/register", post(handlers::register))
        .route("/login", post(handlers::login));

    // 需要认证的认证路由
    let protected_auth_routes = Router::new()
        .route("/me", get(handlers::get_current_user))
        .layer(middleware::from_fn_with_state(
            AuthMiddlewareState {
                auth_config: state.inner.auth_config.clone(),
                required_permission: Permission::Read,
            },
            auth_permission_middleware,
        ));

    // 公开路由（无需认证）
    let public_routes = Router::new()
        // 健康检查
        .route("/health", get(|| async { "OK" }));

    // 文件系统路由 - 每个路由单独配置认证和权限
    // 注意：必须使用 Router::new().route().route_layer() 分组方式
    // 因为 route_layer() 会应用到之前定义的所有路由，而不是仅应用到最近的路由
    let list_routes = Router::new()
        .route("/fs/list", get(handlers::list_files))
        .route("/fs/dir", get(handlers::get_file_info))
        .layer(middleware::from_fn_with_state(
            AuthMiddlewareState {
                auth_config: state.inner.auth_config.clone(),
                required_permission: Permission::List,
            },
            auth_permission_middleware,
        ));

    let download_routes = Router::new()
        .route("/fs/get", get(handlers::get_file))
        .route("/fs/download", get(handlers::download_file))
        .layer(middleware::from_fn_with_state(
            AuthMiddlewareState {
                auth_config: state.inner.auth_config.clone(),
                required_permission: Permission::Download,
            },
            auth_permission_middleware,
        ));

    let upload_routes = Router::new()
        .route("/fs/upload", put(handlers::upload_file))
        .route("/fs/upload-info", post(handlers::get_upload_info))
        .route("/fs/upload/complete", post(handlers::complete_upload))
        .layer(middleware::from_fn_with_state(
            AuthMiddlewareState {
                auth_config: state.inner.auth_config.clone(),
                required_permission: Permission::Upload,
            },
            auth_permission_middleware,
        ));

    let mkdir_routes = Router::new()
        .route("/fs/mkdir", post(handlers::mkdir))
        .layer(middleware::from_fn_with_state(
            AuthMiddlewareState {
                auth_config: state.inner.auth_config.clone(),
                required_permission: Permission::CreateDir,
            },
            auth_permission_middleware,
        ));

    let remove_routes = Router::new()
        .route("/fs/remove", post(handlers::remove))
        .layer(middleware::from_fn_with_state(
            AuthMiddlewareState {
                auth_config: state.inner.auth_config.clone(),
                required_permission: Permission::Delete,
            },
            auth_permission_middleware,
        ));

    let rename_routes = Router::new()
        .route("/fs/rename", post(handlers::rename))
        .layer(middleware::from_fn_with_state(
            AuthMiddlewareState {
                auth_config: state.inner.auth_config.clone(),
                required_permission: Permission::Upload,
            },
            auth_permission_middleware,
        ));

    let copy_routes = Router::new().route("/fs/copy", post(handlers::copy)).layer(
        middleware::from_fn_with_state(
            AuthMiddlewareState {
                auth_config: state.inner.auth_config.clone(),
                required_permission: Permission::Copy,
            },
            auth_permission_middleware,
        ),
    );

    let move_routes = Router::new()
        .route("/fs/move", post(handlers::move_file))
        .layer(middleware::from_fn_with_state(
            AuthMiddlewareState {
                auth_config: state.inner.auth_config.clone(),
                required_permission: Permission::Move,
            },
            auth_permission_middleware,
        ));

    // 合并所有文件系统路由
    let fs_routes = list_routes
        .merge(download_routes)
        .merge(upload_routes)
        .merge(mkdir_routes)
        .merge(remove_routes)
        .merge(rename_routes)
        .merge(copy_routes)
        .merge(move_routes);

    // 需要管理员权限的路由
    let admin_routes = Router::new()
        // 存储管理接口
        .route("/admin/storage/list", get(handlers::list_storages))
        .route("/admin/storage/add", post(handlers::add_storage))
        .route(
            "/admin/storage/delete/{name}",
            delete(handlers::remove_storage),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            admin_auth_middleware,
        ));

    Router::new()
        .nest("/api", public_auth_routes.merge(public_routes))
        // 认证路由（需要 JWT 认证）
        .nest("/api", protected_auth_routes)
        // 文件系统路由（需要 JWT 认证和权限检查）
        .nest("/api", fs_routes)
        // 管理员路由
        .nest("/api", admin_routes)
        .with_state(state)
}
