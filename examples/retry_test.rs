use sage_core::{
    config::provider::ProviderConfig,
    llm::{
        client::LLMClient,
        messages::LLMMessage,
        providers::{LLMProvider, ModelParameters},
    },
};
use tracing_subscriber::fmt::init;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    init();

    // Create a Google provider configuration with retry settings
    let provider_config = ProviderConfig::new("google")
        .with_api_key("AIzaSyCtI947T9sCiW6fMob6Sipt8l0JfGFS_U4")
        .with_timeout(30)
        .with_max_retries(3);

    // Create model parameters
    let model_params = ModelParameters::new("gemini-2.5-pro")
        .with_max_tokens(1000)
        .with_temperature(0.7);

    // Create LLM client
    let client = LLMClient::new(LLMProvider::Google, provider_config, model_params)?;

    // Create a simple message using the helper method
    let messages = vec![LLMMessage::user("Hello, how are you?")];

    println!("Testing LLM client with retry mechanism...");

    // Make the request - this should automatically retry on failure
    match client.chat(&messages, None).await {
        Ok(response) => {
            println!("✅ Request successful!");
            println!("Response: {}", response.content);
        }
        Err(error) => {
            println!("❌ Request failed after retries: {}", error);
        }
    }

    Ok(())
}
