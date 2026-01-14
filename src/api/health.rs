//! Health check endpoints.

use actix_web::{get, web, HttpResponse};
use serde::Serialize;

use crate::config;

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Service status.
    pub status: &'static str,
    /// Service version.
    pub version: &'static str,
    /// Service name.
    pub service: &'static str,
}

/// Readiness check response.
#[derive(Debug, Serialize)]
pub struct ReadyResponse {
    /// Service status.
    pub status: &'static str,
    /// Music folder accessible.
    pub music_folder: bool,
    /// Users file accessible.
    pub users_file: bool,
}

/// Health check endpoint.
///
/// GET /health
///
/// Returns 200 if the service is running.
#[get("/health")]
pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
        service: env!("CARGO_PKG_NAME"),
    })
}

/// Readiness check endpoint.
///
/// GET /ready
///
/// Returns 200 if the service is ready to accept requests.
/// Checks that required resources are accessible.
#[get("/ready")]
pub async fn ready() -> HttpResponse {
    let config = config::get();

    let music_folder_ok = config.music_folder.exists() && config.music_folder.is_dir();
    let users_file_ok = config
        .users_file
        .parent()
        .map(|p| p.exists())
        .unwrap_or(true);

    let all_ok = music_folder_ok && users_file_ok;

    let response = ReadyResponse {
        status: if all_ok { "ready" } else { "not_ready" },
        music_folder: music_folder_ok,
        users_file: users_file_ok,
    };

    if all_ok {
        HttpResponse::Ok().json(response)
    } else {
        HttpResponse::ServiceUnavailable().json(response)
    }
}

/// Configure health routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(health).service(ready);
}
