pub mod config;
pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod state;

use axum::http::StatusCode;
pub use config::ApiConfig;
pub use state::AppState;
use std::time::Duration;
use tower_http::{cors::CorsLayer, services::ServeDir, timeout::TimeoutLayer, trace::TraceLayer};
use tracing::info;

/// 启动 API 服务器
pub async fn start_server(state: AppState, addr: &str) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("API 服务器启动在 {}", addr);
    info!("前端页面访问地址：http://{}/", addr);

    let cors = CorsLayer::permissive();
    let static_service = ServeDir::new("static").append_index_html_on_directories(true);
    let api_routes = routes::create_routes(state.clone())
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::GATEWAY_TIMEOUT,
            Duration::from_secs(30 * 60),
        ));
    let app = api_routes
        .fallback_service(static_service)
        .layer(cors)
        .with_state(state);

    axum::serve(listener, app).await
}
