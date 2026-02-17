use crate::domain::entities::{Command, CommandRegistry, Message, Content};
use crate::application::errors::{CommandError, BotError};

/// Service for managing and executing commands
pub struct CommandService {
    registry: CommandRegistry,
    prefix: String,
}

impl CommandService {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            registry: CommandRegistry::new(),
            prefix: prefix.into(),
        }
    }

    pub fn register(&mut self, command: Command) {
        self.registry.register(command);
    }

    pub fn register_defaults(&mut self) {
        // Help command
        self.register(Command::new("help")
            .with_description("Show help message")
            .with_usage("/help [command]")
            .with_handler(|msg| {
                Ok("Available commands:\n/help - Show this message\n/version - Show version".to_string())
            }));

        // Version command
        self.register(Command::new("version")
            .with_description("Show bot version")
            .with_handler(|_| {
                Ok("carik-bot v0.1.0".to_string())
            }));
    }

    pub fn handle(&self, message: &Message) -> Result<Option<String>, CommandError> {
        let Content::Command { name, args } = &message.content else {
            return Ok(None);
        };

        // Find command (without prefix)
        let cmd = self.registry.find(name)
            .ok_or_else(|| CommandError::NotFound(name.clone()))?;

        // Execute handler
        if let Some(handler) = &cmd.handler {
            Ok(Some(handler(message.clone())?))
        } else {
            Ok(Some(format!("Command {} not implemented", cmd.name)))
        }
    }

    pub fn get_help(&self, command: Option<&str>) -> String {
        if let Some(name) = command {
            if let Some(cmd) = self.registry.get(name) {
                let mut help = format!("/{} - {}", cmd.name, cmd.description.as_deref().unwrap_or("No description"));
                if let Some(usage) = &cmd.usage {
                    help.push_str(&format!("\nUsage: {}", usage));
                }
                return help;
            }
            return format!("Command /{} not found", name);
        }

        // List all commands
        let mut help = "Available commands:\n".to_string();
        for cmd in self.registry.all() {
            help.push_str(&format!("  /{} - {}\n", cmd.name, cmd.description.as_deref().unwrap_or("")));
        }
        help
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }
}
