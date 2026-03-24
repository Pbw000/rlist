pub mod config;
pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod state;

use axum::{
    Router,
    http::StatusCode,
    response::Redirect,
    routing::{get, get_service},
};
pub use config::ApiConfig;
pub use state::AppState;
use std::time::Duration;
use tower_http::{cors::CorsLayer, services::ServeDir, timeout::TimeoutLayer, trace::TraceLayer};
use tracing::info;

/// 启动 API 服务器
pub async fn start_server(state: AppState, addr: &str) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("API 服务器启动在 {}", addr);
    info!("前端页面访问地址：http://{}/public.html", addr);

    let cors = CorsLayer::permissive();
    let static_service = ServeDir::new("static").append_index_html_on_directories(false);

    // 默认根路径重定向到 public.html
    let root_redirect = get(|| async { Redirect::to("/public.html") });

    let api_routes = routes::create_routes(state.clone())
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::GATEWAY_TIMEOUT,
            Duration::from_secs(30 * 60),
        ));
    let app = Router::new()
        .route("/", root_redirect)
        .merge(api_routes)
        .fallback_service(get_service(static_service))
        .layer(cors)
        .with_state(state);

    axum::serve(listener, app).await
}
