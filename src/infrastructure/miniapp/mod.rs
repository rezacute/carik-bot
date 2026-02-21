//! Mini-Apps Plugin System
//! 
//! Lightweight plugins for interactive mini-apps (games, tools, etc.)

use std::collections::HashMap;
use std::sync::Mutex;

/// Mini-app trait - implemented by each mini-app
pub trait MiniApp: Send + Sync {
    /// App name
    fn name(&self) -> &str;
    
    /// Handle incoming message - returns response if handled
    fn handle(&self, text: &str, user_id: &str, state: &mut AppState) -> Option<String>;
    
    /// Get help text
    fn help(&self) -> &str;
}

/// Application state for mini-apps
pub struct AppState {
    pub scramble: Option<ScrambleState>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            scramble: None,
        }
    }
}

/// Scramble game state
pub struct ScrambleState {
    pub word: String,
    pub scrambled: String,
    pub hints_used: u32,
    pub attempts: u32,
}

/// Mini-app manager
pub struct MiniAppManager {
    apps: Vec<Box<dyn MiniApp>>,
}

impl MiniAppManager {
    pub fn new() -> Self {
        let mut manager = Self { apps: Vec::new() };
        manager.register(Box::new(ScrambleApp::new()));
        manager
    }
    
    fn register(&mut self, app: Box<dyn MiniApp>) {
        self.apps.push(app);
    }
    
    /// Try to handle message with any mini-app
    pub fn handle(&self, text: &str, user_id: &str, state: &mut AppState) -> Option<String> {
        let lower = text.to_lowercase();
        let cmd = lower.trim_start_matches('/');
        
        tracing::debug!("MiniAppManager.handle called: text='{}', cmd='{}', apps={}", text, cmd, self.apps.len());
        
        // Check if user wants to start a mini-app or use game commands
        if lower.contains("scramble") || cmd == "scramble" ||
           cmd == "hint" || cmd == "guess" || cmd == "quit" {
            for app in &self.apps {
                tracing::debug!("Delegating to app.handle for '{}'", app.name());
                return app.handle(text, user_id, state);
            }
        }
        
        tracing::debug!("MiniAppManager.handle: no match, returning None");
        None
    }
}

impl Default for MiniAppManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Scramble Game Mini-App
pub struct ScrambleApp {
    words: Vec<(&'static str, &'static str)>,
}

impl ScrambleApp {
    pub fn new() -> Self {
        Self {
            words: vec![
                ("RUST", "Programming language"),
                ("TELEGRAM", "Messaging app"),
                ("PYTHON", "Programming language"),
                ("ROBOT", "Automated machine"),
                ("NETWORK", "Computer system"),
                ("DATABASE", "Data storage"),
                ("ALGORITHM", "Step-by-step procedure"),
                ("INTERFACE", "User connection"),
                ("VARIABLE", "Data container"),
                ("FUNCTION", "Code block"),
                ("KEYBOARD", "Input device"),
                ("MONITOR", "Display device"),
                ("SOFTWARE", "Programs"),
                ("HARDWARE", "Physical parts"),
                ("BROWSER", "Web viewer"),
            ],
        }
    }
    
    fn scramble_word(word: &str) -> String {
        let mut chars: Vec<char> = word.chars().collect();
        // Simple shuffle
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::SystemTime;
        
        let mut hasher = DefaultHasher::new();
        SystemTime::now().hash(&mut hasher);
        let seed = hasher.finish();
        
        // Fisher-Yates shuffle
        for i in (1..chars.len()).rev() {
            let j = ((seed as usize + i * 31) % (i + 1));
            chars.swap(i, j);
        }
        
        let scrambled: String = chars.iter().collect();
        // Make sure it's actually scrambled
        if scrambled == word && word.len() > 1 {
            chars.swap(0, 1);
        }
        chars.iter().collect()
    }
}

impl MiniApp for ScrambleApp {
    fn name(&self) -> &str {
        "scramble"
    }
    
    fn help(&self) -> &str {
        "🎮 Scramble Game\n\n\
Unscramble the letters to find the word!\n\
• /scramble - Start new game\n\
• /hint - Get a hint (max 2)\n\
• /guess [word] - Guess the answer\n\
• /quit - Quit game"
    }
    
    fn handle(&self, text: &str, user_id: &str, state: &mut AppState) -> Option<String> {
        let lower = text.to_lowercase();
        let cmd = lower.trim_start_matches('/');
        
        tracing::debug!("ScrambleApp.handle: text='{}', cmd='{}', has_game={}", text, cmd, state.scramble.is_some());
        
        // Handle hint/guess/quit even without active game
        if cmd == "hint" || cmd == "guess" || cmd == "quit" {
            if state.scramble.is_none() {
                tracing::debug!("ScrambleApp.handle: no active game, returning 'No active game'");
                return Some("🎮 No active game!\n\nUse /scramble to start a new game.".to_string());
            }
        }
        
        // Start new game
        if cmd == "scramble" || lower.contains("play scramble") || lower.contains("main scramble") {
            let idx = rand_index(self.words.len());
            let (word, hint) = self.words[idx];
            let scrambled = Self::scramble_word(word);
            
            state.scramble = Some(ScrambleState {
                word: word.to_string(),
                scrambled: scrambled.clone(),
                hints_used: 0,
                attempts: 0,
            });
            
            return Some(format!(
                "🎮 *Scramble Game!*\n\n\
Unscramble this word:\n\
*`{}`*\n\n\
Reply with your guess or /hint for help.",
                scrambled
            ));
        }
        
        // Check if there's an active game
        if let Some(ref mut game) = state.scramble {
            // Hint
            if cmd == "hint" {
                if game.hints_used < 2 {
                    game.hints_used += 1;
                    let hint_idx = game.hints_used as usize;
                    let word = &game.word;
                    let hint = self.words.iter()
                        .find(|(w, _)| w == word)
                        .map(|(_, h)| h)
                        .unwrap_or(&"");
                    
                    // Show hint based on letters revealed
                    let revealed: String = word.chars()
                        .enumerate()
                        .map(|(i, c)| {
                            if i < game.hints_used as usize || game.hints_used == 0 {
                                "_".to_string()
                            } else {
                                c.to_string()
                            }
                        })
                        .collect();
                    
                    return Some(format!(
                        "💡 Hint {}: *{}*\n\n\
Word: *{}*\n\n\
Scrambled: `{}`",
                        game.hints_used,
                        hint,
                        revealed,
                        game.scrambled
                    ));
                } else {
                    return Some("❌ No more hints available!".to_string());
                }
            }
            
            // Guess
            if cmd == "guess" {
                let guess = text
                    .replace("/guess", "")
                    .replace("guess", "")
                    .trim()
                    .to_uppercase();
                
                game.attempts += 1;
                
                if guess == game.word {
                    let response = format!(
                        "🎉 *Correct!*\n\n\
The word was *{}*\n\
You guessed in *{}* attempt(s)!",
                        game.word,
                        game.attempts
                    );
                    state.scramble = None;
                    return Some(response);
                } else {
                    return Some(format!(
                        "❌ Wrong! Try again.\n\
Attempts: *{}*\n\
Scrambled: `{}`",
                        game.attempts,
                        game.scrambled
                    ));
                }
            }
            
            // Quit
            if cmd == "quit" || lower.contains("quit game") {
                state.scramble = None;
                return Some("👋 Game ended. Thanks for playing!".to_string());
            }
            
            // Show current state
            if cmd == "scramble" || lower == "status" {
                return Some(format!(
                    "📊 Game Status\n\n\
Scrambled: *{}*\n\
Attempts: {}\n\
Hints used: {}/2\n\n\
Use /guess [word] to answer!",
                    game.scrambled,
                    game.attempts,
                    game.hints_used
                ));
            }
        }
        
        None
    }
}

fn rand_index(max: usize) -> usize {
    use std::time::SystemTime;
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    
    let mut hasher = DefaultHasher::new();
    SystemTime::now().hash(&mut hasher);
    let seed = hasher.finish();
    
    (seed as usize) % max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scramble_word() {
        let word = "RUST";
        let scrambled = ScrambleApp::scramble_word(word);
        // Scrambled word should have same length
        assert_eq!(scrambled.len(), word.len());
    }

    #[test]
    fn test_detect_game_commands() {
        let manager = MiniAppManager::new();
        let mut state = AppState::default();
        
        // Test /hint without game - should return "No active game"
        let response = manager.handle("/hint", "user1", &mut state);
        assert!(response.is_some());
        assert!(response.unwrap().contains("No active game"));
        
        // Test /guess without game  
        let response = manager.handle("/guess", "user1", &mut state);
        assert!(response.is_some());
        assert!(response.unwrap().contains("No active game"));
        
        // Test /quit without game
        let response = manager.handle("/quit", "user1", &mut state);
        assert!(response.is_some());
        assert!(response.unwrap().contains("No active game"));
        
        // Now test /scramble - starts game
        let response = manager.handle("/scramble", "user1", &mut state);
        assert!(response.is_some());
        assert!(response.unwrap().contains("Scramble"));
    }

    #[test]
    fn test_game_commands_with_active_game() {
        let manager = MiniAppManager::new();
        let mut state = AppState::default();
        
        // Start game
        let response = manager.handle("/scramble", "user1", &mut state);
        assert!(response.is_some(), "Scramble should return response");
        assert!(state.scramble.is_some(), "Game should be started");
        eprintln!("DEBUG: After scramble, state.scramble = {:?}", state.scramble);
        
        // Test /hint with game
        let response = manager.handle("/hint", "user1", &mut state);
        eprintln!("DEBUG: /hint response = {:?}", response);
        assert!(response.is_some());
        assert!(response.unwrap().contains("Hint"));
        
        // Test correct guess
        let word = state.scramble.as_ref().unwrap().word.clone();
        let guess_cmd = format!("/guess {}", word);
        let response = manager.handle(&guess_cmd, "user1", &mut state);
        assert!(response.is_some());
        assert!(response.unwrap().contains("Correct"));
        
        // Game should be over
        assert!(state.scramble.is_none());
    }

    #[test]
    fn test_cmd_parsing() {
        // Test that cmd extraction works correctly
        let text1 = "/hint";
        let cmd1 = text1.trim_start_matches('/').split_whitespace().next().unwrap_or("");
        assert_eq!(cmd1, "hint");
        
        let text2 = "hint";
        let cmd2 = text2.trim_start_matches('/').split_whitespace().next().unwrap_or("");
        assert_eq!(cmd2, "hint");
        
        let text3 = "/scramble test";
        let cmd3 = text3.trim_start_matches('/').split_whitespace().next().unwrap_or("");
        assert_eq!(cmd3, "scramble");
        
        let text4 = "guess RUST";
        let cmd4 = text4.trim_start_matches('/').split_whitespace().next().unwrap_or("");
        assert_eq!(cmd4, "guess");
    }

    #[test]
    fn test_mini_app_manager() {
        let manager = MiniAppManager::new();
        
        // Should not handle random text
        let state = &mut AppState::default();
        let response = manager.handle("hello world", "user1", state);
        assert!(response.is_none());
        
        // Should handle /scramble
        let response = manager.handle("/scramble", "user1", state);
        assert!(response.is_some());
    }
}
