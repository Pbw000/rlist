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

use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};
use tracing::info;

/// 启动 API 服务器
pub async fn start_server(state: AppState, addr: &str) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("API 服务器启动在 {}", addr);
    info!("前端页面访问地址：http://{}/", addr);

    let cors = CorsLayer::permissive();

    // 静态文件服务
    let static_service = ServeDir::new("static").append_index_html_on_directories(true);

    // API 路由（不包含 with_state）
    let api_routes = routes::create_routes(state.clone())
        .layer(TraceLayer::new_for_http())
        .layer(cors.clone());

    // 合并路由 - 使用 fallback_service 处理静态文件
    let app = api_routes
        .fallback_service(static_service)
        .with_state(state);

    axum::serve(listener, app).await
}
