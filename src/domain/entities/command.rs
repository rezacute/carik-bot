use std::collections::HashMap;

/// Represents a bot command
pub struct Command {
    pub name: String,
    pub description: Option<String>,
    pub aliases: Vec<String>,
    pub usage: Option<String>,
    pub handler: Option<CommandHandler>,
    pub permissions: Vec<String>,
}

/// Command handler function type
pub type CommandHandler = Box<dyn Fn(crate::domain::entities::Message) -> Result<String, crate::application::errors::CommandError> + Send + Sync>;

impl Command {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            aliases: Vec::new(),
            usage: None,
            handler: None,
            permissions: Vec::new(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_aliases(mut self, aliases: Vec<String>) -> Self {
        self.aliases = aliases;
        self
    }

    pub fn with_usage(mut self, usage: impl Into<String>) -> Self {
        self.usage = Some(usage.into());
        self
    }

    pub fn with_permission(mut self, permission: impl Into<String>) -> Self {
        self.permissions.push(permission.into());
        self
    }

    pub fn with_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(crate::domain::entities::Message) -> Result<String, crate::application::errors::CommandError> + Send + Sync + 'static,
    {
        self.handler = Some(Box::new(handler));
        self
    }

    pub fn matches(&self, input: &str) -> bool {
        let input_lower = input.to_lowercase();
        self.name.to_lowercase() == input_lower || 
            self.aliases.iter().any(|a| a.to_lowercase() == input_lower)
    }
}

/// Command registry for managing available commands
#[derive(Default)]
pub struct CommandRegistry {
    commands: HashMap<String, Command>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, command: Command) {
        self.commands.insert(command.name.clone(), command);
    }

    pub fn get(&self, name: &str) -> Option<&Command> {
        self.commands.get(name)
    }

    pub fn find(&self, input: &str) -> Option<&Command> {
        self.commands.values().find(|c| c.matches(input))
    }

    pub fn all(&self) -> impl Iterator<Item = &Command> {
        self.commands.values()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
