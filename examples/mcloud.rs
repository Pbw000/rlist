//! 中国移动云盘示例
//!
//! 使用统一的 Storage trait API

use rlist::storage::driver::mcloud::client::McloudStorage;
use rlist::storage::model::Storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("中国移动云盘示例 - 统一 Storage API\n");

    // 创建 McloudStorage 实例
    // 注意：请替换为您的实际 token
    let storage = McloudStorage::from_authorization("YOUR_MCLOUD_TOKEN_HERE");

    // 使用统一的 Storage trait 方法
    println!("存储名称：{}", storage.name());
    println!("驱动名称：{}\n", storage.driver_name());

    println!("尝试列出根目录文件...");
    match storage.list_files("root", 50, None).await {
        Ok(file_list) => {
            println!(
                "文件总数：{} (当前页：{})",
                file_list.total,
                file_list.items.len()
            );
            for file in file_list.items.iter() {
                let icon = if file.is_dir() { "📁" } else { "📄" };
                let size = if file.is_dir() {
                    "".to_string()
                } else {
                    file.human_size()
                };
                match file {
                    rlist::Meta::File { name, .. } => {
                        println!("  {} {} {}", icon, name, size);
                    }
                    rlist::Meta::Directory { name, .. } => {
                        println!("  {} {} {}", icon, name, size);
                    }
                }
            }
        }
        Err(e) => println!("列出文件失败：{}", e),
    }

    Ok(())
}
