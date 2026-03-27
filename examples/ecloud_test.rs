//! 天翼云盘测试脚本

use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionRequest {
    sessionId: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionResponse {
    res_code: Option<i64>,
    res_message: Option<String>,
    data: Option<SessionData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionData {
    sessionKey: Option<String>,
    sessionSecret: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 使用提供的 Cookie
    let cookie = "JSESSIONID=6867CCCAF2F5C486B96526287DF010AE; COOKIE_LOGIN_USER=4953CB231A034538D136823745C79598D1080E1BC256621A8BED2FD4F383ED7FCF92D553EAF1F707D176B5A8CBBD7EE9";

    let client = Client::builder().build()?;

    // 尝试获取 sessionKey 和 sessionSecret
    // 天翼云盘的 getSessionForPC API
    let url = "https://api.cloud.189.cn/getSessionForPC.action?sessionOption=1";

    let response: reqwest::Response = client
        .get(url)
        .header("Cookie", cookie)
        .header("Accept", "application/json;charset=UTF-8")
        .send()
        .await?;

    println!("Status: {}", response.status());

    let text = response.text().await?;
    println!("Response: {}", text);

    // 尝试解析响应
    if let Ok(session_resp) = serde_json::from_str::<SessionResponse>(&text) {
        if let Some(data) = session_resp.data {
            if let (Some(key), Some(secret)) = (data.sessionKey, data.sessionSecret) {
                println!("\n=== 获取到认证信息 ===");
                println!("session_key: {}", key);
                println!("session_secret: {}", secret);
                return Ok(());
            }
        }
    }

    // 如果上面的方法不行，尝试另一个 API
    println!("\n尝试另一个 API...");
    let url2 = "https://api.cloud.189.cn/v2/getSessionKey.action";
    let response2: reqwest::Response = client
        .get(url2)
        .header("Cookie", cookie)
        .header("Accept", "application/json;charset=UTF-8")
        .send()
        .await?;

    println!("Status: {}", response2.status());
    let text2 = response2.text().await?;
    println!("Response: {}", text2);

    Ok(())
}
