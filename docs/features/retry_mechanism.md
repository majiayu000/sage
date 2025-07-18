# 重试机制 (Retry Mechanism)

Sage Agent 现在包含了一个强大的重试机制，可以自动处理临时的API错误和网络问题。

## 功能特性

### 自动重试
- **指数退避**: 使用 2^attempt 秒的延迟策略
- **智能错误检测**: 只重试可恢复的错误
- **可配置重试次数**: 通过配置文件设置最大重试次数

### 支持的错误类型
重试机制会自动重试以下类型的错误：

- **HTTP 5xx 错误**: 503 (Service Unavailable), 502 (Bad Gateway), 504 (Gateway Timeout)
- **速率限制**: 429 (Too Many Requests)
- **网络错误**: 连接超时、网络中断等
- **服务过载**: API提供商临时过载

### 不会重试的错误
以下错误类型不会触发重试：

- **认证错误**: 401 (Unauthorized), 403 (Forbidden)
- **客户端错误**: 400 (Bad Request), 404 (Not Found)
- **配置错误**: 无效的API密钥、模型名称等

## 配置

### 在配置文件中设置重试次数

```json
{
  "model_providers": {
    "google": {
      "api_key": "your-api-key",
      "model": "gemini-2.5-pro",
      "max_retries": 5,
      "max_tokens": 120000,
      "temperature": 0.7
    },
    "openai": {
      "api_key": "your-api-key", 
      "model": "gpt-4",
      "max_retries": 3,
      "max_tokens": 4096,
      "temperature": 0.7
    }
  }
}
```

### 默认值
- 如果未在配置中指定，默认重试次数为 3 次
- 重试延迟使用指数退避：1秒、2秒、4秒、8秒...

## 使用示例

### CLI 使用
重试机制在CLI中自动启用，无需额外配置：

```bash
# 重试机制会自动处理临时错误
sage run "创建一个Python脚本"
```

### SDK 使用
```rust
use sage_core::{
    config::provider::ProviderConfig,
    llm::{
        client::LLMClient,
        providers::{LLMProvider, ModelParameters},
        messages::LLMMessage,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建配置，设置重试次数
    let provider_config = ProviderConfig::new("google")
        .with_api_key("your-api-key")
        .with_max_retries(5);

    let model_params = ModelParameters::new("gemini-2.5-pro");
    
    let client = LLMClient::new(
        LLMProvider::Google,
        provider_config,
        model_params,
    )?;

    let messages = vec![LLMMessage::user("Hello!")];
    
    // 自动重试失败的请求
    match client.chat(&messages, None).await {
        Ok(response) => println!("成功: {}", response.content),
        Err(error) => println!("重试后仍失败: {}", error),
    }

    Ok(())
}
```

## 日志输出

重试机制会输出详细的日志信息：

```
WARN Request failed (attempt 1/4): Google API error: {"error":{"code":503,"message":"The model is overloaded. Please try again later.","status":"UNAVAILABLE"}}. Retrying in 1 seconds...
WARN Request failed (attempt 2/4): Google API error: {"error":{"code":503,"message":"The model is overloaded. Please try again later.","status":"UNAVAILABLE"}}. Retrying in 2 seconds...
WARN Request failed (attempt 3/4): Google API error: {"error":{"code":503,"message":"The model is overloaded. Please try again later.","status":"UNAVAILABLE"}}. Retrying in 4 seconds...
```

## 最佳实践

1. **合理设置重试次数**: 
   - 对于生产环境，建议设置 3-5 次重试
   - 对于开发环境，可以设置更少的重试次数以快速失败

2. **监控重试日志**: 
   - 频繁的重试可能表明API配额不足或服务问题
   - 使用日志来识别和解决潜在问题

3. **错误处理**: 
   - 即使有重试机制，也要妥善处理最终失败的情况
   - 考虑实现降级策略或用户友好的错误消息

## 技术实现

重试机制在 `LLMClient` 中实现，使用以下策略：

- **指数退避算法**: `delay = 2^attempt` 秒
- **错误分类**: 基于错误消息内容判断是否可重试
- **异步实现**: 使用 `tokio::time::sleep` 进行非阻塞延迟
- **日志记录**: 使用 `tracing` 框架记录重试过程
