//! Ferrum - A lightweight, self-hosted music streaming server.
//!
//! Ferrum provides a REST API for streaming local music files,
//! with JWT-based authentication and multi-user support.

mod api;
mod auth;
mod config;
mod error;
mod models;

use actix_cors::Cors;
use actix_web::{http::header, middleware::Logger, web, App, HttpServer};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::auth::JsonUserRepository;
use crate::config::LogFormat;
use crate::models::AppState;

/// Initialize the tracing/logging subsystem.
fn init_tracing(config: &config::Config) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    let subscriber = tracing_subscriber::registry().with(env_filter);

    match config.log_format {
        LogFormat::Json => {
            subscriber
                .with(tracing_subscriber::fmt::layer().json())
                .init();
        }
        LogFormat::Pretty => {
            subscriber
                .with(tracing_subscriber::fmt::layer().pretty())
                .init();
        }
    }
}

/// Configure CORS based on application config.
fn configure_cors(config: &config::Config) -> Cors {
    let mut cors = Cors::default()
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allowed_headers(vec![
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
        ])
        .max_age(3600);

    if config.cors_origins.len() == 1 && config.cors_origins[0] == "*" {
        cors = cors.allow_any_origin();
    } else {
        for origin in &config.cors_origins {
            cors = cors.allowed_origin(origin);
        }
    }

    cors
}

/// Graceful shutdown handler.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, starting graceful shutdown");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, starting graceful shutdown");
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize configuration
    let config = config::init();

    // Initialize logging
    init_tracing(config);

    // Validate configuration
    if let Err(e) = config.validate() {
        tracing::error!(error = %e, "Configuration validation failed");
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()));
    }

    // Initialize user repository
    let user_repo = Arc::new(
        JsonUserRepository::new(&config.users_file).map_err(|e| {
            tracing::error!(error = %e, "Failed to initialize user repository");
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?,
    );

    // Create application state
    let app_state = AppState {
        music_folder: config.music_folder.clone(),
        user_repo: user_repo.clone(),
    };

    let bind_address = config.bind_address();

    tracing::info!(
        address = %bind_address,
        music_folder = %config.music_folder.display(),
        "Starting Ferrum server"
    );

    // Create and start server
    let server = HttpServer::new(move || {
        App::new()
            // Middleware (order matters - outermost first)
            .wrap(Logger::default())
            .wrap(configure_cors(config))
            // Shared state
            .app_data(web::Data::new(app_state.clone()))
            .app_data(web::Data::from(user_repo.clone()))
            // Health endpoints (no auth required)
            .configure(api::health::configure)
            // Auth endpoints (no auth required for login/register)
            .configure(api::auth::configure)
            // Music endpoints (auth required)
            .configure(api::music::configure)
    })
    .bind(&bind_address)?
    .shutdown_timeout(30)
    .run();

    // Run server with graceful shutdown
    tokio::select! {
        result = server => {
            result
        }
        _ = shutdown_signal() => {
            tracing::info!("Shutdown complete");
            Ok(())
        }
    }
}
