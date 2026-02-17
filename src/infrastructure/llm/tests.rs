//! Integration tests for LLM providers

#[cfg(test)]
mod tests {
    use crate::infrastructure::llm::{LLM, LLMMessage, GroqProvider, LLMConfig, LLMProvider};

    #[tokio::test]
    #[ignore] // Requires GROQ_API_KEY environment variable
    async fn test_groq_chat() {
        let config = LLMConfig::from_env();
        let api_key = config.api_key(LLMProvider::Groq).expect("GROQ_API_KEY not set");
        
        let provider = GroqProvider::new(api_key, None);
        
        let messages = vec![
            LLMMessage::system("You are a helpful assistant."),
            LLMMessage::user("What is 2+2?"),
        ];
        
        let response = provider.chat(messages, None, Some(0.7), Some(100))
            .await
            .expect("Chat request failed");
        
        println!("Response: {}", response.content);
        println!("Model: {}", response.model);
        
        assert!(!response.content.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_groq_with_env_key() {
        // Test using environment variable directly
        let api_key = std::env::var("GROQ_API_KEY").expect("GROQ_API_KEY not set");
        
        // llama-3.1-70b-versatile is deprecated, using llama-3.1-8b-instant
        let provider = GroqProvider::new(api_key, Some("llama-3.1-8b-instant"));
        
        let messages = vec![
            LLMMessage::user("Say 'hello' in exactly one word."),
        ];
        
        let response = provider.chat(messages, None, Some(0.5), Some(10))
            .await
            .expect("Chat request failed");
        
        // Verify response is short
        assert!(response.content.len() < 20, "Response too long: {}", response.content);
    }

    #[test]
    fn test_llm_config_from_env() {
        // Set environment variable for testing
        std::env::set_var("GROQ_API_KEY", "test-key-123");
        
        let config = LLMConfig::from_env();
        
        assert_eq!(config.api_key(LLMProvider::Groq), Some("test-key-123"));
        
        // Clean up
        std::env::remove_var("GROQ_API_KEY");
    }

    #[test]
    fn test_llm_message_builder() {
        let msg = LLMMessage::user("Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
        
        let system_msg = LLMMessage::system("You are helpful.");
        assert_eq!(system_msg.role, "system");
    }
}
