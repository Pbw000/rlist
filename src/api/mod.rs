//! RESTful API 模块
//!
//! 提供类似 AList 的 RESTful API 接口

pub mod config;
pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod state;

pub use config::ApiConfig;
pub use state::AppState;

use axum::Router;
use std::time::Duration;
use tower_http::{cors::CorsLayer, services::ServeDir, timeout::TimeoutLayer, trace::TraceLayer};
use tracing::info;

/// 启动 API 服务器
pub async fn start_server(state: AppState, addr: &str) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("API 服务器启动在 {}", addr);
    info!("前端页面访问地址：http://{}/", addr);

    // 配置 CORS - 允许所有来源
    let cors = CorsLayer::permissive();

    // 静态文件服务
    let static_service = ServeDir::new("static").append_index_html_on_directories(true);

    // API 路由（不包含 with_state）
    // 添加超时层（30 分钟，支持大文件上传）
    let api_routes = routes::create_routes(state.clone())
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(30 * 60)));

    // 合并路由 - 使用 fallback_service 处理静态文件
    // CORS 层应用于所有路由
    let app = api_routes
        .fallback_service(static_service)
        .layer(cors)
        .with_state(state);

    axum::serve(listener, app).await
}
