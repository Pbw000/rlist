use std::path::PathBuf;
use std::sync::OnceLock;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Layer, Registry, filter::LevelFilter};

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub fn init_logging() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let file_appender = create_monthly_rolling_appender();
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // 保存 guard 到全局静态变量
    if let Err(_) = LOG_GUARD.set(guard) {
        eprintln!("Logging already was initialized!");
        return Ok(());
    }

    // 文件日志：只记录 WARN 及以上级别（关键安全事件）
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_filter(LevelFilter::WARN);

    // 控制台日志：记录 INFO 及以上级别
    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_filter(LevelFilter::INFO);

    let subscriber = Registry::default().with(console_layer).with(file_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    Ok(())
}
fn create_monthly_rolling_appender() -> RollingFileAppender {
    let now = chrono::Local::now();
    let year = now.format("%Y").to_string();
    let month = now.format("%-m").to_string(); // %-m 去除前导零 (Linux/Mac)

    // 创建目录结构：logs/{year}/{year}-{month}/
    let log_dir = PathBuf::from("logs").join(&year).join(&month);

    std::fs::create_dir_all(&log_dir).expect("Failed to create log directory");

    // 使用每日滚动，文件名将包含日期
    RollingFileAppender::new(Rotation::DAILY, log_dir, "rlist.log")
}

/// 获取日志文件的基础路径（用于测试或调试）
pub fn get_log_base_path() -> PathBuf {
    let now = chrono::Local::now();
    let year = now.format("%Y").to_string();
    let month = now.format("%-m").to_string();
    PathBuf::from("logs").join(&year).join(&month)
}
