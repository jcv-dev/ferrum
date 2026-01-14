//! Authentication API endpoints.

use actix_web::{get, post, web, HttpResponse};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::auth::{jwt, AuthenticatedUser, JsonUserRepository, User, UserRepository};
use crate::error::{AppError, AppResult};

/// Request body for user registration.
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    /// Username (3-32 characters, alphanumeric and underscores).
    #[validate(length(min = 3, max = 32, message = "Username must be 3-32 characters"))]
    #[validate(regex(
        path = "USERNAME_REGEX",
        message = "Username can only contain letters, numbers, and underscores"
    ))]
    pub username: String,
    /// Password (8-128 characters).
    #[validate(length(min = 8, max = 128, message = "Password must be 8-128 characters"))]
    pub password: String,
}

lazy_static::lazy_static! {
    static ref USERNAME_REGEX: regex::Regex = regex::Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
}

/// Request body for user login.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Response for successful authentication.
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub token: jwt::TokenPair,
}

/// Public user information in responses.
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: uuid::Uuid,
    pub username: String,
    pub is_admin: bool,
    pub created_at: chrono::DateTime<Utc>,
}

impl From<&User> for UserResponse {
    fn from(user: &User) -> Self {
        Self {
            id: user.id,
            username: user.username.clone(),
            is_admin: user.is_admin,
            created_at: user.created_at,
        }
    }
}

/// Hash a password using Argon2.
fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to hash password");
            AppError::Internal("Failed to process password".to_string())
        })
}

/// Verify a password against a hash.
fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
    let parsed_hash = PasswordHash::new(hash).map_err(|e| {
        tracing::error!(error = %e, "Failed to parse password hash");
        AppError::Internal("Failed to verify password".to_string())
    })?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Register a new user.
///
/// POST /auth/register
///
/// The first registered user automatically becomes an admin.
#[post("/register")]
pub async fn register(
    repo: web::Data<JsonUserRepository>,
    body: web::Json<RegisterRequest>,
) -> AppResult<HttpResponse> {
    // Validate input
    body.validate().map_err(|e| {
        AppError::Validation(e.to_string())
    })?;

    // Check if username already exists
    if repo.username_exists(&body.username)? {
        return Err(AppError::Conflict(format!(
            "Username '{}' is already taken",
            body.username
        )));
    }

    // First user becomes admin
    let is_admin = repo.count()? == 0;

    // Hash password
    let password_hash = hash_password(&body.password)?;

    // Create user
    let user = User::new(body.username.clone(), password_hash, is_admin);
    let user = repo.create(user)?;

    // Generate token
    let token = jwt::create_token_pair(user.id, user.username.clone(), user.is_admin)?;

    tracing::info!(
        user_id = %user.id,
        username = %user.username,
        is_admin = user.is_admin,
        "New user registered"
    );

    Ok(HttpResponse::Created().json(AuthResponse {
        user: UserResponse::from(&user),
        token,
    }))
}

/// Login with username and password.
///
/// POST /auth/login
#[post("/login")]
pub async fn login(
    repo: web::Data<JsonUserRepository>,
    body: web::Json<LoginRequest>,
) -> AppResult<HttpResponse> {
    // Find user
    let user = repo
        .find_by_username(&body.username)?
        .ok_or_else(|| AppError::invalid_credentials())?;

    // Verify password
    if !verify_password(&body.password, &user.password_hash)? {
        return Err(AppError::invalid_credentials());
    }

    // Update last login
    let mut updated_user = user.clone();
    updated_user.last_login = Some(Utc::now());
    let _ = repo.update(updated_user);

    // Generate token
    let token = jwt::create_token_pair(user.id, user.username.clone(), user.is_admin)?;

    tracing::info!(user_id = %user.id, username = %user.username, "User logged in");

    Ok(HttpResponse::Ok().json(AuthResponse {
        user: UserResponse::from(&user),
        token,
    }))
}

/// Get current user information.
///
/// GET /auth/me
///
/// Requires authentication.
#[get("/me")]
pub async fn me(
    user: AuthenticatedUser,
    repo: web::Data<JsonUserRepository>,
) -> AppResult<HttpResponse> {
    let user = repo
        .find_by_id(user.id)?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(HttpResponse::Ok().json(UserResponse::from(&user)))
}

/// Configure auth routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .service(register)
            .service(login)
            .service(me),
    );
}
