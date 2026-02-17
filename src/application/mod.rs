//! Application layer - Use cases and business logic
//! 
//! This layer contains:
//! - Services: Business logic orchestration
//! - Commands: CLI command handlers
//! - Errors: Domain-specific errors
//! - Messaging: Message parsing, middleware, dispatching

pub mod errors;
pub mod services;
pub mod messaging;
