//! Domain layer - Core business logic with no external dependencies
//! 
//! This layer contains:
//! - Entities: Core business objects (User, Message, Command)
//! - Traits: Abstractions for infrastructure (Bot, Store)
//! - Rules: Business logic invariants

pub mod entities;
pub mod traits;
