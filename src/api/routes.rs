//! API 路由定义

use axum::{
    Router, middleware,
    routing::{delete, get, post, put},
};

use crate::api::handlers;
use crate::api::middleware::admin_auth_middleware;
use crate::api::state::AppState;

/// 创建 API 路由（不绑定状态）
pub fn create_routes(state: AppState) -> Router<AppState> {
    // 公开路由（无需认证）
    let public_routes = Router::new()
        // 文件系统接口
        .route("/fs/list", get(handlers::list_files))
        .route("/fs/get", get(handlers::get_file))
        .route("/fs/download", get(handlers::download_file))
        .route("/fs/dir", get(handlers::get_file_info))
        .route("/fs/mkdir", post(handlers::mkdir))
        .route("/fs/remove", post(handlers::remove))
        .route("/fs/rename", post(handlers::rename))
        .route("/fs/copy", post(handlers::copy))
        .route("/fs/move", post(handlers::move_file))
        .route("/fs/upload", put(handlers::upload_file))
        .route("/fs/upload-info", get(handlers::get_upload_info))
        // 路径导航接口
        .route("/fs/navigate", post(handlers::navigate_path))
        .route("/fs/parent-dirs", get(handlers::get_parent_dirs));

    // 需要管理员权限的路由（特定路由）
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
        .nest("/api", public_routes.merge(admin_routes))
        // 健康检查（公开）
        .route("/health", get(|| async { "OK" }))
}
