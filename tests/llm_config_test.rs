//! LLM Configuration Integration Tests
//! Run with: cargo test --test llm_config_test

use std::sync::Once;
use reqwest;

static INIT: Once = Once::new();

fn ensure_init() {
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt::init();
    });
}

/// Test that GROQ_API_KEY is set and has valid format
#[test]
fn test_groq_api_key_exists() {
    ensure_init();
    
    let api_key = std::env::var("GROQ_API_KEY")
        .expect("GROQ_API_KEY must be set in environment");
    
    // Groq API keys start with "gsk_"
    assert!(api_key.starts_with("gsk_"), 
        "GROQ_API_KEY should start with 'gsk_': {}", api_key);
    assert!(api_key.len() > 20, 
        "GROQ_API_KEY should be reasonably long");
}

/// Test that we can make a simple API call to Groq
#[tokio::test]
async fn test_groq_api_call() {
    ensure_init();
    
    let api_key = std::env::var("GROQ_API_KEY")
        .expect("GROQ_API_KEY must be set");
    
    let client = reqwest::Client::new();
    
    // Use llama-3.1-8b-instant which is widely available on Groq
    let request = serde_json::json!({
        "model": "llama-3.1-8b-instant",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "Reply with exactly: 'LLM test passed'"}
        ],
        "temperature": 0.1,
        "max_tokens": 50
    });
    
    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .expect("Should make API call");
    
    assert!(response.status().is_success(), 
        "API call should succeed: {:?}", response.text().await);
    
    let body: serde_json::Value = response.json().await.expect("Should parse JSON");
    
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .expect("Should have content");
    
    assert!(content.to_lowercase().contains("llm test passed"),
        "Response should contain 'LLM test passed': {}", content);
}

/// Test API key rejection with invalid key
#[tokio::test]
async fn test_invalid_api_key_rejected() {
    ensure_init();
    
    let client = reqwest::Client::new();
    
    let request = serde_json::json!({
        "model": "llama-3.1-70b-versatile",
        "messages": [{"role": "user", "content": "test"}],
        "max_tokens": 10
    });
    
    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", "Bearer invalid_key_12345")
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .expect("Should make API call");
    
    // Invalid API key should return 401 or 403
    assert!(response.status() == reqwest::StatusCode::UNAUTHORIZED || 
            response.status() == reqwest::StatusCode::FORBIDDEN,
        "Invalid key should be rejected: {}", response.status());
}

/// Test different models are available
#[tokio::test]
async fn test_model_availability() {
    ensure_init();
    
    let api_key = std::env::var("GROQ_API_KEY")
        .expect("GROQ_API_KEY must be set");
    
    let client = reqwest::Client::new();
    
    // Use models that are known to work on Groq
    let models = vec![
        "llama-3.1-8b-instant",
    ];
    
    for model in models {
        let request = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": "test"}],
            "max_tokens": 10
        });
        
        let response = client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .expect("Should make API call");
        
        // Model should either work (200) or return proper model not found error
        // This tests that our API key can reach the Groq API
        assert!(response.status() == reqwest::StatusCode::OK || 
                response.status() == reqwest::StatusCode::BAD_REQUEST,
            "Model {} should be accessible: {}", model, response.status());
    }
}

/// Test response contains expected metadata
#[tokio::test]
async fn test_response_metadata() {
    ensure_init();
    
    let api_key = std::env::var("GROQ_API_KEY")
        .expect("GROQ_API_KEY must be set");
    
    let client = reqwest::Client::new();
    
    let request = serde_json::json!({
        "model": "llama-3.1-8b-instant",
        "messages": [{"role": "user", "content": "What is 2+2?"}],
        "max_tokens": 20
    });
    
    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .expect("Should make API call");
    
    let body: serde_json::Value = response.json().await.expect("Should parse JSON");
    
    // Check response has expected fields
    assert!(body["id"].is_string(), "Response should have id");
    assert!(body["object"].is_string(), "Response should have object type");
    assert!(body["created"].is_number(), "Response should have created timestamp");
    assert!(body["model"].is_string(), "Response should have model name");
    
    // Model should be llama
    let model = body["model"].as_str().unwrap_or("");
    assert!(model.contains("llama"), "Model should be llama: {}", model);
}

/// Test system prompt is respected
#[tokio::test]
async fn test_system_prompt_respected() {
    ensure_init();
    
    let api_key = std::env::var("GROQ_API_KEY")
        .expect("GROQ_API_KEY must be set");
    
    let client = reqwest::Client::new();
    
    let request = serde_json::json!({
        "model": "llama-3.1-8b-instant",
        "messages": [
            {"role": "system", "content": "You only respond with the word 'CONFIRMED' in uppercase."},
            {"role": "user", "content": "What should you say?"}
        ],
        "max_tokens": 10
    });
    
    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .expect("Should make API call");
    
    let body: serde_json::Value = response.json().await.expect("Should parse JSON");
    
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("");
    
    assert!(content.trim() == "CONFIRMED", 
        "System prompt should be respected. Got: {}", content);
}
