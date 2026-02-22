//! Mini-Apps Plugin System
//! 
//! Lightweight plugins for interactive mini-apps (games, tools, etc.)

use std::collections::HashMap;
use std::sync::Mutex;

use crate::infrastructure::llm::{LLMMessage, LLM};

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
    /// LLM-generated creative hint (if using LLM mode)
    pub creative_hint: Option<String>,
    /// Story mode narrative
    pub story: Option<String>,
    /// Word theme/category
    pub theme: Option<String>,
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
        // Use starts_with to handle commands with arguments like "/guess RUST"
        if lower.contains("scramble") || cmd.starts_with("scramble") ||
           cmd.starts_with("hint") || cmd.starts_with("guess") || cmd.starts_with("quit") {
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
    
    /// Generate a word via LLM (for LLM-enhanced mode)
    pub fn generate_word_llm(&self) -> Option<(String, String, String)> {
        // Try to use LLM to generate a word
        // This would require passing the LLM instance, so for now return None
        // In practice, this would be called from main.rs with the LLM provider
        None
    }
    
    /// Generate creative hint via LLM
    pub fn generate_creative_hint_llm(word: &str, hint_num: u32, theme: Option<&str>) -> Option<String> {
        let theme_hint = theme.map(|t| format!("Theme: {}. ", t)).unwrap_or_default();
        
        let prompt = if hint_num == 1 {
            format!("{}Give a creative, fun hint for the word '{}' ({} letters). \
Don't reveal the word! Be playful and indirect. Format: just the hint, nothing else.",
                theme_hint, word, word.len())
        } else {
            format!("{}Give a second, more specific hint for the word '{}' ({} letters). \
This is the second hint, so give more information but still don't reveal. Format: just the hint.",
                theme_hint, word, word.len())
        };
        
        // Use blocking call to get LLM response
        // This requires the LLM to be passed in - placeholder for integration
        tracing::debug!("Would generate creative hint via LLM: {}", prompt);
        None
    }
    
    /// Generate story mode narrative via LLM
    pub fn generate_story_llm(word: &str, attempt: u32, correct: bool) -> Option<String> {
        let prompt = if correct {
            format!("Write a short, fun victory story (2-3 sentences) about guessing the word '{}'. \
Make it celebratory and playful!", word)
        } else if attempt == 1 {
            format!("Write a short, mysterious intro story (2-3 sentences) about a hidden word '{}'. \
Set the mood for a word guessing adventure!", word)
        } else {
            format!("Write a short story snippet (2-3 sentences) for someone trying to guess the word '{}'. \
Be encouraging but mysterious!", word)
        };
        
        tracing::debug!("Would generate story via LLM: {}", prompt);
        None
    }
    
    /// Generate creative hint (static fallback when LLM unavailable)
    fn generate_creative_hint_static(word: &str, hint_num: u32) -> String {
        let hints: Vec<(&str, &str, &str)> = vec![
            ("RUST", "It's what old iron becomes, but also a modern programming language!", "Fear not the crash, this language is known for safety!"),
            ("TELEGRAM", "Not the app, but the tool that changed warfare forever at the Somme.", "It was also a messaging app before there were smartphones!"),
            ("PYTHON", "A snake? No, it's a popular programming language named after a comedy troupe!", "Used by NASA, Netflix, and Instagram!"),
            ("ROBOT", "It comes from a word meaning 'forced labor' in Czech!", "Think Asimov, R2-D2, and ChatGPT's body!"),
            ("NETWORK", "It's not just social - it's how computers talk to each other!", "The internet is the biggest one!"),
            ("DATABASE", "Where your data goes to be organized - think Oracle or MySQL!", "ACID is its middle name!"),
            ("ALGORITHM", "A recipe, but for computers! Named after a Persian mathematician.", "Google uses this to rank websites!"),
            ("INTERFACE", "Where humans meet machines - touchscreens, buttons, or APIs!", "UI and UX are its cousins!"),
            ("VARIABLE", "The 'X' in algebra that computers use to store secrets!", "Let, const, and var are its keywords!"),
            ("FUNCTION", "A reusable code block that does one thing and does it well!", "main(), print(), and your custom functions!"),
            ("KEYBOARD", "Not just for typing - QWERTY was designed to slow you down!", "It has 104 keys on a standard one!"),
            ("MONITOR", "Your window into the digital world - CRT, LED, or OLED!", "144Hz means smooth motion!"),
            ("SOFTWARE", "It's not hard - it's the programs that make hardware dance!", "Windows, macOS, and Linux are its families!"),
            ("MEMORY", "Short-term or long-term - your computer has both!", "RAM is volatile, ROM is not!"),
            ("BINARY", "The language of computers - just 0s and 1s!", "It's base-2, not base-10!"),
            ("SERVER", "A computer that never sleeps, serving data day and night!", "The cloud is just someone else's server!"),
        ];
        
        for (w, h1, h2) in hints {
            if word.eq_ignore_ascii_case(w) {
                return if hint_num == 1 { h1.to_string() } else { h2.to_string() };
            }
        }
        // Default hints for unknown words
        if hint_num == 1 {
            format!("A {}-letter word related to tech!", word.len())
        } else {
            "Keep trying - it's a common computing term!".to_string()
        }
    }
    
    /// Generate story snippet (static fallback)
    fn generate_static_story(word: &str, attempt: u32, correct: bool) -> String {
        if correct {
            format!("🎉 *Victory!* The word was *{}*! Your wits are sharp as a coder's! 🧠💻", word)
        } else if attempt == 0 {
            format!("📖 A mysterious word hides in the shadows... Can you uncover *{}*? The adventure begins! ✨", word.chars().map(|_| '?').collect::<String>())
        } else {
            format!("📖 The mystery deepens... You've tried {} time(s). The word *{}* awaits discovery! 🔍", attempt + 1, word.chars().map(|_| '?').collect::<String>())
        }
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
        
        // Handle hint/guess/quit even without active game - use starts_with for commands with args
        if cmd.starts_with("hint") || cmd.starts_with("guess") || cmd.starts_with("quit") {
            if state.scramble.is_none() {
                tracing::debug!("ScrambleApp.handle: no active game, returning 'No active game'");
                return Some("🎮 No active game!\n\nUse /scramble to start a new game.".to_string());
            }
        }
        
        // Start new game
        if cmd.starts_with("scramble") || lower.contains("play scramble") || lower.contains("main scramble") {
            let idx = rand_index(self.words.len());
            let (word, hint) = self.words[idx];
            let scrambled = Self::scramble_word(word);
            
            state.scramble = Some(ScrambleState {
                word: word.to_string(),
                scrambled: scrambled.clone(),
                hints_used: 0,
                attempts: 0,
                creative_hint: None,
                story: None,
                theme: None,
            });
            
            // Check for story mode
            let story_msg = if lower.contains("story") {
                let story = Self::generate_static_story(word, 0, false);
                state.scramble.as_mut().unwrap().story = Some(story.clone());
                format!("\n\n📖 *Story Mode:* {}", story)
            } else {
                String::new()
            };
            
            return Some(format!(
                "🎮 *Scramble Game!*_{}\n\n\
Unscramble this word:\n\
*`{}`*\n\n\
Reply with your guess or /hint for help.",
                story_msg,
                scrambled
            ));
        }
        
        // Check if there's an active game
        if let Some(ref mut game) = state.scramble {
            // Hint
            if cmd.starts_with("hint") {
                if game.hints_used < 2 {
                    game.attempts += 1;
                    game.hints_used += 1;
                    let hint_idx = game.hints_used as usize;
                    let word = &game.word;
                    
                    // Use creative hint if available (LLM mode), otherwise use static
                    let hint = if let Some(ref creative) = game.creative_hint {
                        creative.clone()
                    } else {
                        // Generate static creative hint
                        Self::generate_creative_hint_static(word, game.hints_used)
                    };
                    
                    // Show hint based on letters revealed (for non-LLM mode)
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
                    
                    // Add story if in story mode
                    let story_msg = if let Some(ref story) = game.story {
                        if game.hints_used == 1 {
                            format!("\n\n📖 *Story continues:* {}", story)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    
                    return Some(format!(
                        "💡 Hint {}: *{}*\n\n\
Word: *{}*\n\n\
Scrambled: `{}`{}",
                        game.hints_used,
                        hint,
                        revealed,
                        game.scrambled,
                        story_msg
                    ));
                } else {
                    return Some("❌ No more hints available!".to_string());
                }
            }
            
            // Guess
            if cmd.starts_with("guess") {
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
            if cmd.starts_with("quit") || lower.contains("quit game") {
                state.scramble = None;
                return Some("👋 Game ended. Thanks for playing!".to_string());
            }
            
            // Show current state
            if cmd.starts_with("scramble") || lower == "status" {
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
