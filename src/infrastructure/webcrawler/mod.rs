//! Web Crawler Module
//! 
//! Smart web crawling with rate limiting and LLM integration

use reqwest::blocking::Client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Rate limiter for domains
pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    min_interval: Duration,
    max_per_minute: u32,
}

impl RateLimiter {
    pub fn new(min_interval_ms: u64, max_per_minute: u32) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            min_interval: Duration::from_millis(min_interval_ms),
            max_per_minute,
        }
    }

    /// Wait if necessary before making a request to this domain
    pub fn wait_for(&self, domain: &str) {
        loop {
            let mut requests = self.requests.lock().unwrap();
            let now = Instant::now();
            
            // Clean old requests (older than 1 minute)
            let one_minute_ago = now - Duration::from_secs(60);
            if let Some(insts) = requests.get_mut(domain) {
                insts.retain(|i| *i > one_minute_ago);
            }
            
            // Check if we've hit the limit
            let count = requests.get(domain).map(|v| v.len()).unwrap_or(0);
            if count >= self.max_per_minute as usize {
                // Wait until oldest request expires
                if let Some(insts) = requests.get(domain) {
                    if let Some(oldest) = insts.first() {
                        let wait_time = one_minute_ago.duration_since(*oldest);
                        if !wait_time.is_zero() {
                            drop(requests);
                            std::thread::sleep(wait_time);
                            continue; // Retry
                        }
                    }
                }
            }
            
            // Add current request
            requests.entry(domain.to_string()).or_insert_with(Vec::new).push(now);
            break;
        }
    }
}

/// Smart web crawler with LLM integration
pub struct WebCrawler {
    client: Client,
    rate_limiter: RateLimiter,
    llm_enabled: bool,
}

impl WebCrawler {
    pub fn new(rate_limit_ms: u64, max_per_minute: u32, llm_enabled: bool) -> Result<Self, String> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        Ok(Self {
            client,
            rate_limiter: RateLimiter::new(rate_limit_ms, max_per_minute),
            llm_enabled,
        })
    }

    /// Fetch a URL with rate limiting
    pub fn fetch(&self, url: &str) -> Result<String, String> {
        // Extract domain for rate limiting
        let domain = extract_domain(url)?;
        
        // Wait for rate limit
        self.rate_limiter.wait_for(&domain);
        
        // Make request
        let response = self.client
            .get(url)
            .header("Accept", "text/html,application/xhtml+xml")
            .send()
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let html = response.text().map_err(|e| e.to_string())?;
        
        // Extract main content
        Ok(extract_content(&html))
    }

    /// Fetch and use LLM to extract relevant content
    pub fn fetch_smart(&self, url: &str, query: &str) -> Result<String, String> {
        // First fetch the page
        let content = self.fetch(url)?;
        
        if !self.llm_enabled || content.len() < 500 {
            // No LLM or content too short - return raw content
            return Ok(content);
        }

        // For now, return truncated content (LLM integration would go here)
        // The LLM would analyze the content and extract what's relevant to query
        let preview = &content[..content.len().min(2000)];
        Ok(format!("[Content from {}]\n\n{}\n\n(Truncated - {} bytes total)", 
            url, preview, content.len()))
    }
}

/// Extract domain from URL
fn extract_domain(url: &str) -> Result<String, String> {
    url.split('/')
        .nth(2)
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid URL".to_string())
}

/// Extract main content from HTML (simple parser)
fn extract_content(html: &str) -> String {
    let mut text = html.to_string();
    
    // Remove script and style tags (simple approach)
    while let Some(start) = text.find("<script") {
        if let Some(end) = text[start..].find("</script>") {
            text = format!("{}{}", &text[..start], &text[start + end + 9..]);
        } else {
            break;
        }
    }
    
    while let Some(start) = text.find("<style") {
        if let Some(end) = text[start..].find("</style>") {
            text = format!("{}{}", &text[..start], &text[start + end + 8..]);
        } else {
            break;
        }
    }
    
    // Remove HTML comments
    while let Some(start) = text.find("<!--") {
        if let Some(end) = text[start..].find("-->") {
            text = format!("{}{}", &text[..start], &text[start + end + 3..]);
        } else {
            break;
        }
    }
    
    // Remove all HTML tags
    let mut in_tag = false;
    let mut result = String::new();
    
    for ch in text.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    
    // Clean up whitespace
    let mut clean = String::new();
    let mut last_space = false;
    
    for ch in result.chars() {
        if ch.is_whitespace() {
            if !last_space {
                clean.push(' ');
                last_space = true;
            }
        } else {
            clean.push(ch);
            last_space = false;
        }
    }
    
    clean.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("https://example.com/page").unwrap(), "example.com");
        assert_eq!(extract_domain("http://news.example.org/path").unwrap(), "news.example.org");
    }

    #[test]
    fn test_extract_content() {
        let html = "<html><head><title>Test</title></head><body><p>Hello World</p></body></html>";
        let content = extract_content(html);
        assert!(content.contains("Hello World"));
    }
}
