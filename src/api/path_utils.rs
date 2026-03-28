//! API 路径工具
//! 处理用户根目录路径前缀

use axum::extract::Request;

/// 根据用户根目录处理路径
/// 如果用户有根目录限制，自动添加前缀
/// 如果没有根目录限制，返回原路径
pub fn apply_user_root_dir(request: &Request, path: &str) -> String {
    // 从请求扩展中获取用户根目录
    if let Some(root_dir) = request.extensions().get::<String>() {
        // 根目录已经是类似 "/user1" 的格式
        // 路径是类似 "/movies/file.mp4" 的格式
        // 合并为 "/user1/movies/file.mp4"
        let path_trimmed = path.trim_start_matches('/');
        if path_trimmed.is_empty() {
            root_dir.clone()
        } else {
            format!("{}/{}", root_dir.trim_end_matches('/'), path_trimmed)
        }
    } else {
        // 没有根目录限制，返回原路径
        path.to_string()
    }
}

/// 从用户路径中移除根目录前缀（用于返回给前端）
pub fn strip_user_root_dir(request: &Request, full_path: &str) -> String {
    // 从请求扩展中获取用户根目录
    if let Some(root_dir) = request.extensions().get::<String>() {
        let root_dir_trimmed = root_dir.trim_end_matches('/');
        if full_path.starts_with(root_dir_trimmed) {
            let remaining = &full_path[root_dir_trimmed.len()..];
            if remaining.is_empty() {
                "/".to_string()
            } else {
                remaining.to_string()
            }
        } else {
            full_path.to_string()
        }
    } else {
        full_path.to_string()
    }
}
