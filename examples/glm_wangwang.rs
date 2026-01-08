use anyhow::Result;
use sage_core::mcp::client::McpClient;
use sage_core::mcp::transport::http::{HttpTransport, HttpTransportConfig};
use sage_core::mcp::types::McpContent;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    println!("🎉 智谱旺旺兑换码获取工具");
    println!("================================\n");

    // 创建 HTTP 传输配置
    let mut config = HttpTransportConfig::new("https://open.bigmodel.cn/api/mcp/glm_camp/mcp");
    config.headers.insert(
        "Authorization".to_string(),
        "Bearer 919c526ea26f48a3a6a843bcfebef277.2Rk8xZA9hqCHSA54".to_string(),
    );
    config.timeout_secs = 300;

    // 创建 HTTP 传输层
    let transport = HttpTransport::new(config)?;

    // 创建 MCP 客户端
    let mut client = McpClient::new(Box::new(transport));

    println!("📡 正在连接到智谱 GLM Camp MCP 服务器...");
    client.initialize().await?;
    println!("✅ 连接成功！\n");

    // 列出可用工具
    println!("📋 正在获取可用工具列表...");
    let tools = client.list_tools().await?;

    println!("✅ 找到 {} 个可用工具:", tools.len());
    for tool in &tools {
        println!("  - {}: {}", tool.name, tool.description.as_deref().unwrap_or("无描述"));
    }
    println!();

    // 调用领取旺仔牛奶的工具
    println!("🎁 正在调用 claim_glm_camp_coupon 工具...");
    let call_response = client
        .call_tool("claim_glm_camp_coupon", serde_json::json!({}))
        .await?;

    println!("================================");
    println!("🎉 领取结果:");
    println!("================================\n");

    // 打印返回的内容
    for content in &call_response.content {
        match content {
            McpContent::Text { text } => {
                println!("{}", text);
            }
            _ => {
                println!("其他类型内容: {:?}", content);
            }
        }
    }

    println!("\n================================");

    // 检查是否有错误
    if call_response.is_error {
        println!("⚠️  领取过程中出现错误");
    } else {
        println!("✅ 兑换码获取成功！");
    }

    Ok(())
}
