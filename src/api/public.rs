//! 公开处理器 - 无需认证即可访问（使用 Challenge 验证）

use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use ring::digest::{Context, SHA512};

use crate::FileList;
use crate::api::{state::AppState, types::*};
use crate::auth::challenge::IntoHashContext;
use crate::storage::model::Storage;

use crate::api::types::ApiResponse;

impl IntoHashContext for PublicFsRequest {
    fn hash_and_to_context(&self) -> Context {
        let mut context = Context::new(&SHA512);
        context.update(self.nonce.as_bytes());
        if let Some(ref path) = self.path {
            context.update(path.as_bytes());
        }
        context.update(&self.timestamp.to_be_bytes());
        context
    }
}

/// 公开列表文件（使用 Challenge 验证，无需登录）
pub async fn public_list_files(
    State(state): State<AppState>,
    Json(payload): Json<PublicFsRequest>,
) -> impl IntoResponse {
    let challenge = &state.inner.challenge;

    if let Err(err) = challenge.validate_timestamp(payload.timestamp) {
        return ApiResponse::error(400, format!("时间戳无效：{}", err));
    }

    if challenge
        .validate::<_, 3>(payload.salt, &payload.claim, &payload)
        .await
        .is_err()
    {
        return ApiResponse::error(400, "Challenge 挑战失败".to_string());
    }

    let path = payload.path.as_deref().unwrap_or("/");
    let registry_guard = state.inner.public_registry.read().await;

    let cursor = payload.cursor.or_else(|| payload.page.map(|p| p as usize));
    let page_size = payload.per_page.unwrap_or(FileList::DEFAULT_PAGE_SIZE);

    match registry_guard.list_files(path, page_size, cursor).await {
        Ok(list) => ApiResponse::success(list),
        Err(e) => ApiResponse::error(500, format!("列出文件失败：{}", e)),
    }
}

/// 公开下载文件（使用 Challenge 验证，无需登录）
pub async fn public_download_file(
    State(state): State<AppState>,
    Json(payload): Json<PublicFsRequest>,
) -> impl IntoResponse {
    let challenge = &state.inner.challenge;

    if let Err(err) = challenge.validate_timestamp(payload.timestamp) {
        return ApiResponse::error(400, format!("时间戳无效：{}", err));
    }

    if challenge
        .validate::<_, 3>(payload.salt, &payload.claim, &payload)
        .await
        .is_err()
    {
        return ApiResponse::error(400, "Challenge 挑战失败".to_string());
    }

    let path = payload.path.as_deref().unwrap_or("/");
    let registry_guard = state.inner.public_registry.read().await;

    match registry_guard.get_download_meta_by_path(path).await {
        Ok(meta) => {
            let resp = FileResponse {
                name: path.split('/').next_back().unwrap_or("unknown").to_string(),
                url: meta.download_url,
                size: meta.size,
                hash: meta.hash,
            };
            ApiResponse::success(resp)
        }
        Err(e) => ApiResponse::error(404, format!("获取下载链接失败：{}", e)),
    }
}
