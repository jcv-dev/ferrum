//! Application configuration management.
//!
//! Loads configuration from environment variables with sensible defaults.

use std::path::PathBuf;
use std::sync::OnceLock;

/// Global configuration instance.
static CONFIG: OnceLock<Config> = OnceLock::new();

/// Application configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Host address to bind to.
    pub host: String,
    /// Port to listen on.
    pub port: u16,
    /// Path to the music folder.
    pub music_folder: PathBuf,
    /// Path to the users JSON file.
    pub users_file: PathBuf,
    /// JWT secret key for signing tokens.
    pub jwt_secret: String,
    /// JWT token expiry in days.
    pub jwt_expiry_days: i64,
    /// Log level (trace, debug, info, warn, error).
    pub log_level: String,
    /// Log format (json or pretty).
    pub log_format: LogFormat,
    /// Allowed CORS origins (comma-separated, or * for all).
    pub cors_origins: Vec<String>,
}

/// Log output format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable colored output.
    Pretty,
    /// JSON structured logging for production.
    Json,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// # Panics
    /// Panics if required configuration is missing or invalid.
    pub fn from_env() -> Self {
        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .expect("PORT must be a valid u16");

        let music_folder = PathBuf::from(
            std::env::var("MUSIC_FOLDER").unwrap_or_else(|_| "./music".to_string()),
        );

        let users_file = PathBuf::from(
            std::env::var("USERS_FILE").unwrap_or_else(|_| "./data/users.json".to_string()),
        );

        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            tracing::warn!(
                "JWT_SECRET not set, using random secret. Tokens will be invalidated on restart!"
            );
            uuid::Uuid::new_v4().to_string()
        });

        let jwt_expiry_days = std::env::var("JWT_EXPIRY_DAYS")
            .unwrap_or_else(|_| "7".to_string())
            .parse::<i64>()
            .expect("JWT_EXPIRY_DAYS must be a valid integer");

        let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        let log_format = match std::env::var("LOG_FORMAT")
            .unwrap_or_else(|_| "pretty".to_string())
            .to_lowercase()
            .as_str()
        {
            "json" => LogFormat::Json,
            _ => LogFormat::Pretty,
        };

        let cors_origins = std::env::var("CORS_ORIGINS")
            .unwrap_or_else(|_| "*".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            host,
            port,
            music_folder,
            users_file,
            jwt_secret,
            jwt_expiry_days,
            log_level,
            log_format,
            cors_origins,
        }
    }

    /// Validate the configuration.
    ///
    /// # Errors
    /// Returns an error if validation fails.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !self.music_folder.exists() {
            return Err(ConfigError::MusicFolderNotFound(
                self.music_folder.display().to_string(),
            ));
        }

        if !self.music_folder.is_dir() {
            return Err(ConfigError::MusicFolderNotDirectory(
                self.music_folder.display().to_string(),
            ));
        }

        if self.jwt_secret.len() < 32 {
            tracing::warn!(
                "JWT_SECRET is shorter than 32 characters. Consider using a longer secret."
            );
        }

        // Ensure users file parent directory exists
        if let Some(parent) = self.users_file.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ConfigError::DataDirectoryCreationFailed(parent.display().to_string(), e)
                })?;
            }
        }

        Ok(())
    }

    /// Get the server bind address.
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Music folder not found: {0}")]
    MusicFolderNotFound(String),

    #[error("Music folder is not a directory: {0}")]
    MusicFolderNotDirectory(String),

    #[error("Failed to create data directory '{0}': {1}")]
    DataDirectoryCreationFailed(String, std::io::Error),
}

/// Initialize the global configuration.
///
/// Should be called once at application startup.
pub fn init() -> &'static Config {
    CONFIG.get_or_init(|| {
        dotenvy::dotenv().ok();
        Config::from_env()
    })
}

/// Get the global configuration.
///
/// # Panics
/// Panics if configuration has not been initialized.
pub fn get() -> &'static Config {
    CONFIG.get().expect("Configuration not initialized. Call config::init() first.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        std::env::remove_var("HOST");
        std::env::remove_var("PORT");
        std::env::remove_var("LOG_LEVEL");

        let config = Config::from_env();

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.log_level, "info");
        assert_eq!(config.jwt_expiry_days, 7);
    }

    #[test]
    fn test_cors_origins_parsing() {
        std::env::set_var("CORS_ORIGINS", "http://localhost:3000, http://example.com");

        let config = Config::from_env();

        assert_eq!(config.cors_origins.len(), 2);
        assert!(config.cors_origins.contains(&"http://localhost:3000".to_string()));
        assert!(config.cors_origins.contains(&"http://example.com".to_string()));

        std::env::remove_var("CORS_ORIGINS");
    }
}
