use rusqlite::{Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub telegram_id: String,
    pub username: Option<String>,
    pub role: String, // owner, admin, user, guest
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitEntry {
    pub id: i64,
    pub user_id: i64,
    pub timestamp: String,
    pub query_type: String, // minute, hour
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: impl AsRef<Path>) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }
    
    fn init_tables(&self) -> SqliteResult<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                telegram_id TEXT UNIQUE NOT NULL,
                username TEXT,
                role TEXT NOT NULL DEFAULT 'guest',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )?;
        
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS rate_limits (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                timestamp TEXT NOT NULL DEFAULT (datetime('now')),
                query_type TEXT NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users(id)
            )",
            [],
        )?;
        
        // User settings table for personalization
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS user_settings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL UNIQUE,
                language TEXT DEFAULT 'en',
                timezone TEXT DEFAULT 'UTC',
                system_prompt TEXT,
                preferences TEXT DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id)
            )",
            [],
        )?;
        
        // Create indexes
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_rate_limits_user ON rate_limits(user_id)",
            [],
        )?;
        
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_rate_limits_timestamp ON rate_limits(timestamp)",
            [],
        )?;
        
        Ok(())
    }
    
    // User management
    pub fn add_user(&self, telegram_id: &str, username: Option<&str>, role: &str) -> SqliteResult<i64> {
        self.conn.execute(
            "INSERT OR REPLACE INTO users (telegram_id, username, role) VALUES (?1, ?2, ?3)",
            rusqlite::params![telegram_id, username.unwrap_or(""), role],
        )?;
        Ok(self.conn.last_insert_rowid())
    }
    
    pub fn get_user_by_telegram_id(&self, telegram_id: &str) -> SqliteResult<Option<User>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, telegram_id, username, role, created_at FROM users WHERE telegram_id = ?1"
        )?;
        
        let mut rows = stmt.query([telegram_id])?;
        
        if let Some(row) = rows.next()? {
            Ok(Some(User {
                id: row.get(0)?,
                telegram_id: row.get(1)?,
                username: row.get(2)?,
                role: row.get(3)?,
                created_at: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }
    
    pub fn list_users(&self) -> SqliteResult<Vec<User>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, telegram_id, username, role, created_at FROM users ORDER BY role, created_at"
        )?;
        
        let rows = stmt.query_map([], |row| {
            Ok(User {
                id: row.get(0)?,
                telegram_id: row.get(1)?,
                username: row.get(2)?,
                role: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        
        let mut users = Vec::new();
        for user in rows {
            users.push(user?);
        }
        Ok(users)
    }
    
    pub fn remove_user(&self, telegram_id: &str) -> SqliteResult<bool> {
        let rows = self.conn.execute(
            "DELETE FROM users WHERE telegram_id = ?1",
            [telegram_id],
        )?;
        Ok(rows > 0)
    }
    
    pub fn update_user_role(&self, telegram_id: &str, role: &str) -> SqliteResult<bool> {
        let rows = self.conn.execute(
            "UPDATE users SET role = ?1 WHERE telegram_id = ?2",
            [role, telegram_id],
        )?;
        Ok(rows > 0)
    }
    
    // Rate limiting
    pub fn record_query(&self, user_id: i64, query_type: &str) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT INTO rate_limits (user_id, query_type) VALUES (?1, ?2)",
            rusqlite::params![user_id, query_type],
        )?;
        Ok(())
    }
    
    pub fn count_recent_queries(&self, user_id: i64, query_type: &str, minutes: i64) -> SqliteResult<i64> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM rate_limits 
             WHERE user_id = ?1 AND query_type = ?2 
             AND timestamp > datetime('now', ?3)"
        )?;
        
        let count: i64 = stmt.query_row(
            rusqlite::params![user_id, query_type, format!("-{} minutes", minutes)],
            |row| row.get(0),
        )?;
        
        Ok(count)
    }
    
    pub fn count_hourly_queries(&self, user_id: i64, query_type: &str) -> SqliteResult<i64> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM rate_limits 
             WHERE user_id = ?1 AND query_type = ?2 
             AND timestamp > datetime('now', '-1 hour')"
        )?;
        
        let count: i64 = stmt.query_row(
            rusqlite::params![user_id, query_type],
            |row| row.get(0),
        )?;
        
        Ok(count)
    }
    
    pub fn cleanup_old_rate_limits(&self) -> SqliteResult<()> {
        // Clean up rate limits older than 1 hour
        self.conn.execute(
            "DELETE FROM rate_limits WHERE timestamp < datetime('now', '-1 hour')",
            [],
        )?;
        Ok(())
    }
    
    // User settings
    pub fn get_user_settings(&self, telegram_id: &str) -> SqliteResult<Option<UserSettings>> {
        // First check if user exists
        let user_exists = self.conn.query_row(
            "SELECT COUNT(*) FROM users WHERE telegram_id = ?1",
            [telegram_id],
            |row| row.get::<_, i32>(0)
        ).unwrap_or(0) > 0;
        
        if !user_exists {
            return Ok(None);
        }
        
        let mut stmt = self.conn.prepare(
            "SELECT us.language, us.timezone, us.system_prompt, us.preferences 
             FROM user_settings us
             JOIN users u ON u.id = us.user_id
             WHERE u.telegram_id = ?1"
        )?;
        
        let result = stmt.query_row([telegram_id], |row| {
            Ok(UserSettings {
                language: row.get(0)?,
                timezone: row.get(1)?,
                system_prompt: row.get(2)?,
                preferences: row.get(3)?,
            })
        });
        
        match result {
            Ok(settings) => Ok(Some(settings)),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // User exists but no settings - return default
                Ok(Some(UserSettings {
                    language: "en".to_string(),
                    timezone: "UTC".to_string(),
                    system_prompt: None,
                    preferences: "{}".to_string(),
                }))
            }
            Err(e) => Err(e),
        }
    }
    
    pub fn set_user_settings(&self, telegram_id: &str, settings: &UserSettings) -> SqliteResult<()> {
        // Ensure user exists - create if not
        let user_id: i64 = match self.conn.query_row(
            "SELECT id FROM users WHERE telegram_id = ?1",
            [telegram_id],
            |row| row.get(0)
        ) {
            Ok(id) => id,
            Err(_) => {
                // Create user with guest role
                self.conn.execute(
                    "INSERT INTO users (telegram_id, role) VALUES (?1, 'guest')",
                    [telegram_id],
                )?;
                self.conn.last_insert_rowid()
            }
        };
        
        self.conn.execute(
            "INSERT OR REPLACE INTO user_settings (user_id, language, timezone, system_prompt, preferences, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
            rusqlite::params![
                user_id,
                &settings.language,
                &settings.timezone,
                &settings.system_prompt.as_deref().unwrap_or(""),
                &settings.preferences
            ],
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserSettings {
    pub language: String,
    pub timezone: String,
    pub system_prompt: Option<String>,
    pub preferences: String,
}
