//! Message dispatcher - Routes messages to handlers

use std::sync::Arc;
use crate::domain::entities::{Message, Content, Command};
use crate::application::errors::BotError;
use super::parser::MessageParser;
use super::middleware::{Context, Middleware, Next, MiddlewareChain, MiddlewareError};

/// Handler function type
pub type Handler = Arc<dyn Fn(Context) -> HandlerResult + Send + Sync>;

/// Handler result
pub type HandlerResult = Result<String, BotError>;

/// Default command handlers
pub struct CommandHandler {
    commands: std::collections::HashMap<String, Command>,
}

impl CommandHandler {
    pub fn new() -> Self {
        let mut commands = std::collections::HashMap::new();
        
        // Help command
        let help_cmd = Command::new("help")
            .with_description("Show help message")
            .with_handler(|_ctx| {
                Ok("Available commands:\n/help - Show this message\n/version - Show version".to_string())
            });
        commands.insert("help".to_string(), help_cmd);
        
        // Version command
        let version_cmd = Command::new("version")
            .with_description("Show bot version")
            .with_handler(|_ctx| {
                Ok("carik-bot v0.1.0".to_string())
            });
        commands.insert("version".to_string(), version_cmd);
        
        Self { commands }
    }

    pub fn register(&mut self, command: Command) {
        self.commands.insert(command.name.clone(), command);
    }

    pub fn handle(&self, name: &str, _ctx: &Context) -> HandlerResult {
        if let Some(cmd) = self.commands.get(name) {
            if let Some(handler) = &cmd.handler {
                return handler(_ctx.message.clone())
                    .map_err(BotError::Command);
            }
        }
        Ok(format!("Unknown command: /{}", name))
    }
}

impl Default for CommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Message dispatcher - routes messages through middleware to handlers
pub struct MessageDispatcher {
    parser: MessageParser,
    middleware: Vec<Arc<dyn Middleware>>,
    command_handler: CommandHandler,
}

impl MessageDispatcher {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            parser: MessageParser::new(prefix),
            middleware: Vec::new(),
            command_handler: CommandHandler::new(),
        }
    }

    /// Add middleware to the chain
    pub fn with_middleware<M: Middleware + 'static>(mut self, middleware: M) -> Self {
        self.middleware.push(Arc::new(middleware));
        self
    }

    /// Register a command handler
    pub fn register_command(&mut self, command: Command) {
        self.command_handler.register(command);
    }

    /// Process a raw text message
    pub fn process_text(&self, chat_id: impl Into<String>, text: impl Into<String>) -> HandlerResult {
        let message = self.parser.parse(chat_id, text, None);
        self.process(message)
    }

    /// Process a message through the dispatcher
    pub fn process(&self, message: Message) -> HandlerResult {
        let ctx = Context::new(message);
        
        // Build middleware chain
        let chain = MiddlewareChain::new()
            .build();
        
        let next = Next::new(chain);
        
        // Run through middleware
        let result = self.run_handler(ctx, next);
        
        match result {
            Ok(ctx) => Ok(ctx.data.get("response").cloned().unwrap_or_default()),
            Err(MiddlewareError::Blocked(msg)) => Ok(msg),
            Err(MiddlewareError::RateLimited { .. }) => Ok("Rate limited. Please try again later.".to_string()),
            Err(MiddlewareError::PermissionDenied(msg)) => Ok(format!("Permission denied: {}", msg)),
            Err(MiddlewareError::Internal(msg)) => Err(BotError::Internal(msg)),
        }
    }

    /// Run the actual handler after middleware
    fn run_handler(&self, ctx: Context, _next: Next) -> Result<Context, MiddlewareError> {
        // Extract command name from message
        if let Content::Command { name, args: _ } = &ctx.message.content {
            // Handle the command
            match self.command_handler.handle(name, &ctx) {
                Ok(response) => {
                    let mut ctx = ctx;
                    ctx.set("response", response);
                    Ok(ctx)
                }
                Err(e) => Err(MiddlewareError::Internal(e.to_string())),
            }
        } else {
            // Echo or ignore non-command messages
            let mut ctx = ctx;
            if let Content::Text(text) = &ctx.message.content {
                ctx.set("response", format!("Echo: {}", text));
            }
            Ok(ctx)
        }
    }
}
