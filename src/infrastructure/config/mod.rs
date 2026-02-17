//! Configuration management

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::application::errors::ConfigError;

/// Bot configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub bot: BotConfig,
    pub plugins: PluginConfig,
    pub security: SecurityConfig,
    pub adapters: AdaptersConfig,
    pub whitelist: WhitelistConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct BotConfig {
    pub name: String,
    pub prefix: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PluginConfig {
    pub directory: PathBuf,
    pub auto_load: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SecurityConfig {
    pub rate_limit: RateLimitConfig,
    pub sandbox: SandboxConfig,
    pub audit: AuditConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RateLimitConfig {
    pub max_requests: u32,
    pub window_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SandboxConfig {
    pub enabled: bool,
    pub memory_mb: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AuditConfig {
    pub enabled: bool,
    pub path: Option<PathBuf>,
}

/// Whitelist configuration for user access control
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct WhitelistConfig {
    pub enabled: bool,
    pub users: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AdaptersConfig {
    pub telegram: Option<TelegramConfig>,
    pub console: Option<ConsoleConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TelegramConfig {
    pub enabled: bool,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ConsoleConfig {
    pub enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bot: BotConfig {
                name: "carik-bot".to_string(),
                prefix: "/".to_string(),
            },
            plugins: PluginConfig {
                directory: PathBuf::from("./plugins"),
                auto_load: true,
            },
            security: SecurityConfig {
                rate_limit: RateLimitConfig {
                    max_requests: 20,
                    window_seconds: 60,
                },
                sandbox: SandboxConfig {
                    enabled: true,
                    memory_mb: Some(256),
                },
                audit: AuditConfig {
                    enabled: true,
                    path: Some(PathBuf::from("logs/audit.log")),
                },
            },
            adapters: AdaptersConfig {
                telegram: Some(TelegramConfig {
                    enabled: false,
                    token: None,
                }),
                console: Some(ConsoleConfig {
                    enabled: true,
                }),
            },
            whitelist: WhitelistConfig {
                enabled: true,
                users: vec!["6504720757".to_string()],
            },
        }
    }
}

impl Config {
    pub fn load(path: impl Into<PathBuf>) -> Result<Self, ConfigError> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)
            .map_err(|e| ConfigError::Parse(format!("Failed to read config: {}", e)))?;

        serde_yaml::from_str(&content)
            .map_err(|e| ConfigError::Parse(format!("Failed to parse config: {}", e)))
    }

    /// Check if a user ID is whitelisted
    pub fn is_user_whitelisted(&self, user_id: &str) -> bool {
        if !self.whitelist.enabled {
            return true; // Whitelist disabled, allow all
        }
        self.whitelist.users.contains(&user_id.to_string())
    }

    pub fn load_env() -> Self {
        // Load from environment variables
        let mut config = Config::default();

        if let Ok(token) = std::env::var("BOT_TOKEN") {
            if let Some(ref mut tg) = config.adapters.telegram {
                tg.token = Some(token);
                tg.enabled = true;
            }
        }

        if let Ok(prefix) = std::env::var("BOT_PREFIX") {
            config.bot.prefix = prefix;
        }

        config
    }
}
