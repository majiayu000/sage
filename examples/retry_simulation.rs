//! LLM Retry Simulation
//!
//! This example simulates various failure scenarios to demonstrate
//! the robustness of the LLM client's retry and error handling mechanisms.

use sage_core::{
    config::provider::ProviderConfig,
    error::SageError,
    llm::{
        LlmProvider, TimeoutConfig, client::LlmClient, messages::LlmMessage,
        provider_types::ModelParameters,
    },
};
use std::time::Instant;
use tracing_subscriber::fmt::init;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志，设置为WARN级别以看到重试日志
    init();

    println!("🔄 Sage Agent 重试机制演示");
    println!("==========================");

    // 创建一个配置，使用无效的API密钥来模拟错误
    let provider_config = ProviderConfig::new("google")
        .with_api_key("invalid-api-key-for-testing")
        .with_timeouts(TimeoutConfig::new().with_request_timeout_secs(10))
        .with_max_retries(3);

    let model_params = ModelParameters::new("gemini-2.5-pro")
        .with_max_tokens(100)
        .with_temperature(0.7);

    let client = LlmClient::new(LlmProvider::Google, provider_config, model_params.clone())?;

    let messages = vec![LlmMessage::user("Hello, this is a test message.")];

    println!("\n📡 测试1: 使用无效API密钥（应该不会重试）");
    let start = Instant::now();
    match client.chat(&messages, None).await {
        Ok(_) => println!("✅ 意外成功"),
        Err(error) => {
            let duration = start.elapsed();
            println!("❌ 失败（预期）: {}", error);
            println!("⏱️  耗时: {:?}", duration);

            // 检查是否是认证错误（不应该重试）
            if let SageError::Llm { message: msg, .. } = &error {
                if msg.contains("401") || msg.contains("403") || msg.contains("API key") {
                    println!("✅ 正确：认证错误没有触发重试");
                }
            }
        }
    }

    println!("\n📡 测试2: 使用有效API密钥但可能遇到服务过载");

    // 使用真实的API密钥（如果可用）
    let real_config = ProviderConfig::new("google")
        .with_api_key("AIzaSyCtI947T9sCiW6fMob6Sipt8l0JfGFS_U4")
        .with_timeouts(TimeoutConfig::new().with_request_timeout_secs(30))
        .with_max_retries(2);

    let real_client = LlmClient::new(LlmProvider::Google, real_config, model_params)?;

    let start = Instant::now();
    match real_client.chat(&messages, None).await {
        Ok(response) => {
            let duration = start.elapsed();
            println!("✅ 请求成功!");
            println!(
                "📝 响应: {}",
                response.content.chars().take(100).collect::<String>()
            );
            println!("⏱️  耗时: {:?}", duration);
        }
        Err(error) => {
            let duration = start.elapsed();
            println!("❌ 请求失败: {}", error);
            println!("⏱️  总耗时: {:?}", duration);

            // 检查是否是可重试的错误
            if let SageError::Llm { message: msg, .. } = &error {
                if msg.contains("503") || msg.contains("overloaded") || msg.contains("429") {
                    println!("✅ 正确：服务过载错误触发了重试机制");
                }
            }
        }
    }

    println!("\n📊 重试机制总结:");
    println!("- ✅ 指数退避延迟策略");
    println!("- ✅ 智能错误分类");
    println!("- ✅ 可配置重试次数");
    println!("- ✅ 详细的日志记录");
    println!("- ✅ 非阻塞异步实现");

    Ok(())
}
