//! 中国移动云盘使用示例
//!
//! 使用统一的 Storage trait API

use rlist::storage::driver::mcloud::client::McloudStorage;
use rlist::storage::model::Storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("中国移动云盘示例 - 统一 Storage API\n");

    // 创建 McloudStorage 实例
    let storage = McloudStorage::from_authorization(
        "cGM6MTM4ODA2MzAzMDk6ZEpjUHBYbEx8MXxSQ1N8MTc3NjY3NTUyMzc3N3xiT1hRRTZ2eUdKSWYuUnVTd3RHdlY1NWo2N2t5Z2NidTNtbnVuUW5sWTRQRDNicm01aWo2VjB6NmcxWm0wQzBDOG5Qdkl6VWhDMjgzc3NrTjdyOFI2eTJYelQxX3pQenJkdE8zbzNQX2s4V2FKUEFnLnNoemY2MHF0VHJRcU9iWUhaVU4wUlI3T1BkNzYxS2pEUS5fTEdfNGhaYUIuWjJ0T05KakRESEIxQTQt",
    )?;

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
                let icon = if file.file_type == rlist::FileType::Directory {
                    "📁"
                } else {
                    "📄"
                };
                let size = if file.file_type == rlist::FileType::Directory {
                    "".to_string()
                } else {
                    file.human_size()
                };
                println!("  {} {} {}", icon, file.name, size);
            }
        }
        Err(e) => println!("列出文件失败：{}", e),
    }

    Ok(())
}
