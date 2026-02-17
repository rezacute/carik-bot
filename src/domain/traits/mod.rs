//! Domain traits - Abstractions for infrastructure implementations

pub mod bot;
pub mod store;

pub use bot::{Bot, BotInfo, KeyboardButton};
pub use store::Store;
