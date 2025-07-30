use actix_web::{get, web, HttpRequest, HttpResponse, Result};
use actix_files::NamedFile;
use crate::models::{AppState, SongMetadata};
use lofty::{read_from_path};
use lofty::prelude::Accessor; 
use lofty::file::{AudioFile, TaggedFileExt}; 
use lofty::picture::PictureType;
use std::fs;

#[get("/api/music/list")]
pub async fn list_music(data: web::Data<AppState>) -> Result<HttpResponse> {
    let mut songs = vec![];
    
    for entry in fs::read_dir(&data.music_folder)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            if let Ok(tagged_file) = read_from_path(&path) {
                let tag = tagged_file.first_tag();
                
                songs.push(SongMetadata {
                    title: tag
                        .and_then(|t| t.title())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Unknown".to_string()),
                    artist: tag
                        .and_then(|t| t.artist())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Unknown".to_string()),
                    album: tag
                        .and_then(|t| t.album())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Unknown".to_string()),
                    duration: Some(tagged_file.properties().duration().as_secs() as u32),
                    file: path.file_name().unwrap().to_string_lossy().into_owned(),
                });
            }
        }
    }
    
    Ok(HttpResponse::Ok().json(songs))
}

#[get("/api/music/stream/{filename}")]
pub async fn stream_music(
    req: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let filename = path.into_inner();
    let full_path = data.music_folder.join(filename);
    
    Ok(NamedFile::open(full_path)?.into_response(&req))
}

#[get("/api/music/cover/{filename}")]
pub async fn get_cover(
    data: web::Data<AppState>,
    path: web::Path<String>
) -> Result<HttpResponse> {
    let filename = path.into_inner();
    let file_path = data.music_folder.join(&filename);
    if let Ok(tagged_file) = read_from_path(&file_path) {
        if let Some(tag) = tagged_file.first_tag() {
            // Find the front cover picture specifically
            if let Some(picture) = tag.pictures().iter().find(|p| p.pic_type() == PictureType::CoverFront) {
                // Fix #2: Handle the Option and provide a default MIME type
                let mime = picture.mime_type()
                    .map(|m| m.as_str())
                    .unwrap_or("application/octet-stream");

                let data = picture.data().to_vec();
                return Ok(HttpResponse::Ok().content_type(mime).body(data));
            }
        }
    }

    Ok(HttpResponse::NotFound().finish())
}
