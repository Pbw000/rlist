//! API 路由定义

use axum::{
    Router, middleware,
    routing::{delete, get, post, put},
};

use crate::api::middleware::admin_permission_middleware;
use crate::api::state::AppState;
use crate::api::{admin, public, user};
use crate::auth::auth::Permission;
use crate::auth::middleware::{AuthMiddlewareState, auth_permission_middleware};

pub fn create_routes(state: AppState) -> Router<AppState> {
    let public_auth_routes = Router::new()
        .route("/login", post(public::login))
        .route("/challenge", get(public::get_challenge));
    let protected_auth_routes = Router::new()
        .route("/me", get(user::get_current_user))
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
        .route("/health", get(|| async { "OK" }))
        // 公开存储访问端点
        .route("/list", post(public::public_list_files))
        .route("/download", get(public::public_download_file));

    let list_routes = Router::new()
        .route("/fs/list", post(public::list_files))
        .route("/fs/dir", get(public::get_file_info))
        .layer(middleware::from_fn_with_state(
            AuthMiddlewareState {
                auth_config: state.inner.auth_config.clone(),
                required_permission: Permission::List,
            },
            auth_permission_middleware,
        ));

    let download_routes = Router::new()
        .route("/fs/get", get(user::get_file))
        .route("/fs/download", get(user::download_file))
        .layer(middleware::from_fn_with_state(
            AuthMiddlewareState {
                auth_config: state.inner.auth_config.clone(),
                required_permission: Permission::Download,
            },
            auth_permission_middleware,
        ));

    let upload_routes = Router::new()
        .route("/fs/upload", put(public::upload_file))
        .route("/fs/upload-info", post(public::get_upload_info))
        .route("/fs/upload/complete", post(public::complete_upload))
        .layer(middleware::from_fn_with_state(
            AuthMiddlewareState {
                auth_config: state.inner.auth_config.clone(),
                required_permission: Permission::Upload,
            },
            auth_permission_middleware,
        ));

    let mkdir_routes = Router::new().route("/fs/mkdir", post(public::mkdir)).layer(
        middleware::from_fn_with_state(
            AuthMiddlewareState {
                auth_config: state.inner.auth_config.clone(),
                required_permission: Permission::CreateDir,
            },
            auth_permission_middleware,
        ),
    );

    let remove_routes = Router::new()
        .route("/fs/remove", post(public::remove))
        .layer(middleware::from_fn_with_state(
            AuthMiddlewareState {
                auth_config: state.inner.auth_config.clone(),
                required_permission: Permission::Delete,
            },
            auth_permission_middleware,
        ));

    let rename_routes = Router::new()
        .route("/fs/rename", post(public::rename))
        .layer(middleware::from_fn_with_state(
            AuthMiddlewareState {
                auth_config: state.inner.auth_config.clone(),
                required_permission: Permission::Upload,
            },
            auth_permission_middleware,
        ));

    let copy_routes =
        Router::new()
            .route("/fs/copy", post(public::copy))
            .layer(middleware::from_fn_with_state(
                AuthMiddlewareState {
                    auth_config: state.inner.auth_config.clone(),
                    required_permission: Permission::Copy,
                },
                auth_permission_middleware,
            ));

    let move_routes = Router::new()
        .route("/fs/move", post(public::move_file))
        .layer(middleware::from_fn_with_state(
            AuthMiddlewareState {
                auth_config: state.inner.auth_config.clone(),
                required_permission: Permission::Move,
            },
            auth_permission_middleware,
        ));
    let public_download_routes = Router::new()
        .route("/public/fs/get", get(public::get_file))
        .route("/public/fs/download", get(public::download_file));
    // 合并所有文件系统路由
    let fs_routes = list_routes
        .merge(download_routes)
        .merge(public_download_routes)
        .merge(upload_routes)
        .merge(mkdir_routes)
        .merge(remove_routes)
        .merge(rename_routes)
        .merge(copy_routes)
        .merge(move_routes);

    // 需要管理员权限的路由
    let admin_routes = Router::new()
        .route("/admin/user/register", post(admin::register))
        .route("/admin/user/list", get(admin::list_users))
        .route("/admin/user/remove", post(admin::remove_user))
        .route(
            "/admin/user/permissions",
            post(admin::update_user_permissions),
        )
        // 存储管理路由
        .route("/admin/storage/list", get(user::list_storages))
        .route("/admin/storage/drivers", get(admin::get_storage_drivers))
        .route(
            "/admin/storage/template/{driver}",
            get(admin::get_storage_template),
        )
        .route("/admin/storage/add", post(admin::add_storage))
        .route(
            "/admin/storage/pub/delete/{index}",
            delete(admin::remove_pub_storage),
        )
        .route(
            "/admin/storage/private/delete/{index}",
            delete(admin::remove_private_storage),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            admin_permission_middleware,
        ));

    Router::new()
        .nest("/api", public_auth_routes)
        .nest("/obs", public_routes)
        .nest("/api", protected_auth_routes)
        .nest("/api", fs_routes)
        .nest("/api", admin_routes)
        .with_state(state)
}
