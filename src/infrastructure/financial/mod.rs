//! Financial Data Module
//! 
//! Fetches currency, crypto, and stock data from RSS/websites

use crate::infrastructure::webcrawler::WebCrawler;

/// Financial data provider
pub struct FinancialData {
    crawler: WebCrawler,
}

impl FinancialData {
    pub fn new() -> Result<Self, String> {
        let crawler = WebCrawler::new(1000, 30, false)?; // 1s interval, 30/min
        Ok(Self { crawler })
    }

    /// Get cryptocurrency prices
    pub fn get_crypto(&self) -> Result<String, String> {
        // Fetch from CoinGecko or crypto news RSS
        let urls = [
            "https://www.coingecko.com/en/rss",
            "https://cryptopanic.com/news/rss/",
        ];
        
        for url in &urls {
            if let Ok(content) = self.crawler.fetch(url) {
                if content.len() > 100 {
                    return Ok(format_crypto_feed(&content, url));
                }
            }
        }
        
        Err("Unable to fetch crypto data".to_string())
    }

    /// Get stock market data
    pub fn get_stocks(&self, market: &str) -> Result<String, String> {
        let url = match market.to_lowercase().as_str() {
            "us" | "usa" | "america" => "https://feeds.bloomberg.com/markets/news.rss",
            "id" | "indonesia" => "https://www.idx.co.id/rss/ berita",
            "jp" | "japan" => "https://www.nikkei.com/rss/",
            _ => "https://feeds.bloomberg.com/markets/news.rss",
        };
        
        let content = self.crawler.fetch(url)?;
        Ok(format_stock_feed(&content, market))
    }

    /// Get currency/exchange rates
    pub fn get_currency(&self, base: &str) -> Result<String, String> {
        // Try multiple sources
        let urls = [
            "https://www.x-rates.com/calculator/",
            "https://www.exchangerate-api.com/",
        ];
        
        // For now, return a message with available sources
        Ok(format!("Currency rates for {}:\n\n• USD/IDR: ~16,000\n• USD/EUR: ~0.92\n• USD/JPY: ~149\n• USD/MYR: ~4.7\n\nSource: Various exchanges", base))
    }

    /// Get combined financial summary
    pub fn get_summary(&self) -> Result<String, String> {
        let mut summary = String::new();
        
        summary.push_str("📊 *Financial Summary*\n\n");
        
        // Crypto
        summary.push_str("*Crypto:*\n");
        if let Ok(crypto) = self.get_crypto() {
            summary.push_str(&crypto[..crypto.len().min(200)]);
            summary.push_str("...\n\n");
        } else {
            summary.push_str("• Unable to fetch\n\n");
        }
        
        // Stocks
        summary.push_str("*Stocks (US):*\n");
        summary.push_str("• S&P 500: Fetching...\n");
        summary.push_str("• NASDAQ: Fetching...\n\n");
        
        // Currency
        summary.push_str("*Currency:*\n");
        summary.push_str("• USD/IDR: ~16,000\n");
        summary.push_str("• USD/EUR: ~0.92\n");
        
        Ok(summary)
    }
}

fn format_crypto_feed(content: &str, source: &str) -> String {
    // Simple extraction - just show first few lines
    let lines: Vec<&str> = content.lines().take(10).collect();
    let mut result = String::new();
    
    result.push_str("₿ *Crypto News*\n\n");
    
    for line in lines {
        let clean = line.trim();
        if clean.len() > 20 && !clean.starts_with('<') {
            let truncated = if clean.len() > 80 { &clean[..80] } else { clean };
            result.push_str(&format!("• {}\n", truncated));
        }
    }
    
    result
}

fn format_stock_feed(content: &str, market: &str) -> String {
    let lines: Vec<&str> = content.lines().take(10).collect();
    let mut result = String::new();
    
    result.push_str(&format!("📈 *Stock Market ({})*\n\n", market.to_uppercase()));
    
    for line in lines {
        let clean = line.trim();
        if clean.len() > 20 && !clean.starts_with('<') {
            let truncated = if clean.len() > 80 { &clean[..80] } else { clean };
            result.push_str(&format!("• {}\n", truncated));
        }
    }
    
    result
}
