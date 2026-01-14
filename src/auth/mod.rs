//! Authentication and authorization module.

pub mod jwt;
pub mod middleware;
pub mod user_repository;

pub use middleware::AuthenticatedUser;
pub use user_repository::{JsonUserRepository, User, UserRepository};
