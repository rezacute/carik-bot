//! Middleware system for message processing pipeline

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use crate::domain::entities::Message;
use crate::application::errors::BotError;

/// Context passed through middleware chain
#[derive(Debug, Clone)]
pub struct Context {
    pub message: Message,
    pub chat_id: String,
    pub user_id: Option<String>,
    pub data: HashMap<String, String>,
}

impl Context {
    pub fn new(message: Message) -> Self {
        let chat_id = message.chat_id.clone();
        let user_id = message.sender.as_ref().map(|u| u.id.clone());
        
        Self {
            message,
            chat_id,
            user_id,
            data: HashMap::new(),
        }
    }

    /// Get data from context
    pub fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }

    /// Set data in context
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.data.insert(key.into(), value.into());
    }
}

/// Middleware trait - processors that can intercept and modify message handling
pub trait Middleware: Send + Sync {
    /// Process a message and optionally modify the context
    fn process(&self, ctx: Context, next: Next) -> MiddlewareResult;
}

/// Result of middleware processing
pub type MiddlewareResult = Result<Context, MiddlewareError>;

/// Middleware errors
#[derive(Debug, Clone)]
pub enum MiddlewareError {
    /// Stop processing and return error
    Blocked(String),
    /// Rate limited
    RateLimited { retry_after: Duration },
    /// Permission denied
    PermissionDenied(String),
    /// Internal error
    Internal(String),
}

impl std::fmt::Display for MiddlewareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MiddlewareError::Blocked(msg) => write!(f, "Blocked: {}", msg),
            MiddlewareError::RateLimited { retry_after } => {
                write!(f, "Rate limited, retry after {:?}", retry_after)
            }
            MiddlewareError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            MiddlewareError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for MiddlewareError {}

/// Next middleware in chain
#[derive(Clone)]
pub struct Next {
    remaining: Arc<Vec<Arc<dyn Middleware>>>,
}

impl Next {
    pub fn new(middlewares: Vec<Arc<dyn Middleware>>) -> Self {
        Self {
            remaining: Arc::new(middlewares),
        }
    }

    /// Process remaining middleware
    pub fn run(self, mut ctx: Context) -> MiddlewareResult {
        if let Some(first) = self.remaining.first() {
            let remaining = self.remaining[1..].to_vec();
            let next = Next::new(remaining);
            first.process(ctx, next)
        } else {
            // No more middleware, processing complete
            Ok(ctx)
        }
    }
}

/// Middleware chain builder
pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    pub fn add<M: Middleware + 'static>(mut self, middleware: M) -> Self {
        self.middlewares.push(Arc::new(middleware));
        self
    }

    pub fn build(self) -> Vec<Arc<dyn Middleware>> {
        self.middlewares
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Rate limit middleware
pub struct RateLimitMiddleware {
    requests: std::sync::Mutex<HashMap<String, Vec<Instant>>>,
    max_requests: u32,
    window: Duration,
}

impl RateLimitMiddleware {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            requests: std::sync::Mutex::new(HashMap::new()),
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }

    fn check_rate_limit(&self, key: &str) -> Result<(), MiddlewareError> {
        let mut requests = self.requests.lock()
            .map_err(|_| MiddlewareError::Internal("Lock poisoned".to_string()))?;
        
        let now = Instant::now();
        
        // Get or create entry
        let times = requests.entry(key.to_string()).or_insert_with(Vec::new);
        
        // Remove old requests outside the window
        times.retain(|&t| now.duration_since(t) < self.window);
        
        // Check limit
        if times.len() >= self.max_requests as usize {
            let retry_after = times.first()
                .map(|t| self.window.saturating_sub(now.duration_since(*t)))
                .unwrap_or(self.window);
            
            return Err(MiddlewareError::RateLimited { retry_after });
        }
        
        // Add current request
        times.push(now);
        Ok(())
    }
}

impl Middleware for RateLimitMiddleware {
    fn process(&self, mut ctx: Context, next: Next) -> MiddlewareResult {
        // Rate limit by user or chat
        let key = ctx.user_id.clone()
            .or_else(|| Some(ctx.chat_id.clone()))
            .unwrap_or_default();
        
        self.check_rate_limit(&key)?;
        
        next.run(ctx)
    }
}

/// Logging middleware for debugging
pub struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn process(&self, ctx: Context, next: Next) -> MiddlewareResult {
        let msg_preview = ctx.message.content.text()
            .map(|s| s.chars().take(50).collect::<String>())
            .unwrap_or_else(|| "[command]".to_string());
        
        tracing::debug!("[{}] {}", ctx.chat_id, msg_preview);
        
        let result = next.run(ctx.clone());
        
        match &result {
            Ok(_) => {
                tracing::debug!("[{}] Processed OK", ctx.chat_id);
            }
            Err(e) => {
                tracing::warn!("[{}] Error: {}", ctx.chat_id, e);
            }
        }
        
        result
    }
}
