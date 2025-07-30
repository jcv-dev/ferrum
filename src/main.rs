mod api;
mod models;

use actix_web::{web, App, HttpServer};
use api::music::{list_music, stream_music};
use std::path::PathBuf;
use models::AppState;

use crate::api::music::get_cover;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let music_folder = std::env::var("MUSIC_FOLDER").unwrap_or_else(|_| "./music".to_string());
    let app_state = AppState {
        music_folder: PathBuf::from(music_folder),
    };

    println!("Server running on http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .service(list_music)
            .service(stream_music)
            .service(get_cover)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
