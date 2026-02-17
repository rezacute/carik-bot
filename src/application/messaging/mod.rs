//! Message handling - Event-driven message processing

pub mod dispatcher;
pub mod middleware;
pub mod parser;

pub use dispatcher::MessageDispatcher;
pub use middleware::{Middleware, MiddlewareChain, RateLimitMiddleware};
pub use parser::MessageParser;
