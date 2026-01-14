//! Music API endpoints.

use actix_files::NamedFile;
use actix_web::{get, http::header, web, HttpRequest, HttpResponse};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::PictureType;
use lofty::prelude::Accessor;
use lofty::read_from_path;
use std::fs;
use std::path::Path;

use crate::auth::AuthenticatedUser;
use crate::error::{AppError, AppResult};
use crate::models::{
    AppState, ListSongsQuery, PaginatedResponse, SongMetadata, SortField, SortOrder,
};

/// Validate and sanitize a filename to prevent path traversal attacks.
///
/// Returns an error if the filename contains path traversal sequences.
fn sanitize_filename(filename: &str) -> AppResult<&str> {
    // Reject empty filenames
    if filename.is_empty() {
        return Err(AppError::BadRequest("Filename cannot be empty".to_string()));
    }

    // Reject path traversal attempts
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        tracing::warn!(filename = %filename, "Path traversal attempt blocked");
        return Err(AppError::path_traversal());
    }

    // Reject absolute paths (Unix and Windows)
    if filename.starts_with('/') || filename.chars().nth(1) == Some(':') {
        return Err(AppError::path_traversal());
    }

    Ok(filename)
}

/// Extract song metadata from an audio file.
fn extract_metadata(path: &Path) -> Option<SongMetadata> {
    let tagged_file = read_from_path(path).ok()?;
    let tag = tagged_file.first_tag();
    let properties = tagged_file.properties();

    let filename = path.file_name()?.to_string_lossy().into_owned();
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| "unknown".to_string());

    let has_cover = tag
        .map(|t| {
            t.pictures()
                .iter()
                .any(|p| p.pic_type() == PictureType::CoverFront)
        })
        .unwrap_or(false);

    Some(SongMetadata {
        id: SongMetadata::generate_id(path),
        title: tag
            .and_then(|t| t.title())
            .map(|s| s.to_string())
            .unwrap_or_else(|| filename.clone()),
        artist: tag
            .and_then(|t| t.artist())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown Artist".to_string()),
        album: tag
            .and_then(|t| t.album())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown Album".to_string()),
        duration: Some(properties.duration().as_secs() as u32),
        track_number: tag.and_then(|t| t.track()),
        year: tag.and_then(|t| t.year()).map(|y| y as i32),
        genre: tag.and_then(|t| t.genre()).map(|s| s.to_string()),
        format: extension,
        file: filename,
        has_cover,
    })
}

/// Supported audio file extensions.
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "wav", "m4a", "aac", "wma", "opus", "aiff", "ape",
];

/// Check if a file has a supported audio extension.
fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// List all songs in the music library with filtering, sorting, and pagination.
///
/// GET /api/music/list
///
/// Query parameters:
/// - `q`: Search query (searches title, artist, album)
/// - `artist`: Filter by artist name
/// - `album`: Filter by album name
/// - `genre`: Filter by genre
/// - `page`: Page number (default: 1)
/// - `per_page`: Items per page (default: 50, max: 100)
/// - `sort`: Sort field (title, artist, album, year, duration)
/// - `order`: Sort order (asc, desc)
#[get("/api/music/list")]
pub async fn list_music(
    _user: AuthenticatedUser,
    data: web::Data<AppState>,
    query: web::Query<ListSongsQuery>,
) -> AppResult<HttpResponse> {
    let query = query.into_inner();

    // Clamp per_page to reasonable limits
    let per_page = query.per_page.clamp(1, 100);
    let page = query.page.max(1);

    // Scan directory for audio files
    let mut songs: Vec<SongMetadata> = fs::read_dir(&data.music_folder)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_audio_file(path))
        .filter_map(|path| extract_metadata(&path))
        .collect();

    // Apply search filter
    if let Some(ref q) = query.q {
        let q_lower = q.to_lowercase();
        songs.retain(|s| {
            s.title.to_lowercase().contains(&q_lower)
                || s.artist.to_lowercase().contains(&q_lower)
                || s.album.to_lowercase().contains(&q_lower)
        });
    }

    // Apply artist filter
    if let Some(ref artist) = query.artist {
        let artist_lower = artist.to_lowercase();
        songs.retain(|s| s.artist.to_lowercase().contains(&artist_lower));
    }

    // Apply album filter
    if let Some(ref album) = query.album {
        let album_lower = album.to_lowercase();
        songs.retain(|s| s.album.to_lowercase().contains(&album_lower));
    }

    // Apply genre filter
    if let Some(ref genre) = query.genre {
        let genre_lower = genre.to_lowercase();
        songs.retain(|s| {
            s.genre
                .as_ref()
                .map(|g| g.to_lowercase().contains(&genre_lower))
                .unwrap_or(false)
        });
    }

    // Sort songs
    songs.sort_by(|a, b| {
        let cmp = match query.sort {
            SortField::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
            SortField::Artist => a.artist.to_lowercase().cmp(&b.artist.to_lowercase()),
            SortField::Album => a.album.to_lowercase().cmp(&b.album.to_lowercase()),
            SortField::Year => a.year.cmp(&b.year),
            SortField::Duration => a.duration.cmp(&b.duration),
        };

        match query.order {
            SortOrder::Asc => cmp,
            SortOrder::Desc => cmp.reverse(),
        }
    });

    // Paginate
    let total = songs.len();
    let start = (page - 1) * per_page;
    let paginated_songs: Vec<SongMetadata> = songs.into_iter().skip(start).take(per_page).collect();

    let response = PaginatedResponse::from_vec(paginated_songs, page, per_page, total);

    Ok(HttpResponse::Ok().json(response))
}

/// Stream an audio file.
///
/// GET /api/music/stream/{filename}
///
/// Supports range requests for seeking.
#[get("/api/music/stream/{filename}")]
pub async fn stream_music(
    req: HttpRequest,
    _user: AuthenticatedUser,
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let filename = sanitize_filename(&path)?;
    let full_path = data.music_folder.join(filename);

    // Check file exists
    if !full_path.exists() {
        return Err(AppError::song_not_found(filename));
    }

    // Verify the resolved path is still within music folder (extra safety)
    let canonical = full_path
        .canonicalize()
        .map_err(|_| AppError::song_not_found(filename))?;
    let music_canonical = data
        .music_folder
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("Music folder error: {}", e)))?;

    if !canonical.starts_with(&music_canonical) {
        tracing::warn!(
            requested = %canonical.display(),
            music_folder = %music_canonical.display(),
            "Path escape attempt blocked"
        );
        return Err(AppError::path_traversal());
    }

    let file = NamedFile::open(&full_path)?;
    Ok(file.into_response(&req))
}

/// Get album cover art for a track.
///
/// GET /api/music/cover/{filename}
///
/// Returns the embedded cover art if available, with caching headers.
#[get("/api/music/cover/{filename}")]
pub async fn get_cover(
    _user: AuthenticatedUser,
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let filename = sanitize_filename(&path)?;
    let file_path = data.music_folder.join(filename);

    // Check file exists
    if !file_path.exists() {
        return Err(AppError::song_not_found(filename));
    }

    let tagged_file =
        read_from_path(&file_path).map_err(|_| AppError::song_not_found(filename))?;

    let tag = tagged_file
        .first_tag()
        .ok_or_else(|| AppError::NotFound("No cover art available".to_string()))?;

    // Find front cover or any picture
    let picture = tag
        .pictures()
        .iter()
        .find(|p| p.pic_type() == PictureType::CoverFront)
        .or_else(|| tag.pictures().first())
        .ok_or_else(|| AppError::NotFound("No cover art available".to_string()))?;

    let mime = picture
        .mime_type()
        .map(|m| m.as_str())
        .unwrap_or("image/jpeg");

    let data = picture.data().to_vec();

    // Cache cover art for 1 day (it rarely changes)
    Ok(HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, mime))
        .insert_header((header::CACHE_CONTROL, "public, max-age=86400"))
        .body(data))
}

/// Get unique artists in the library.
///
/// GET /api/music/artists
#[get("/api/music/artists")]
pub async fn list_artists(
    _user: AuthenticatedUser,
    data: web::Data<AppState>,
) -> AppResult<HttpResponse> {
    let mut artists: Vec<String> = fs::read_dir(&data.music_folder)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_audio_file(path))
        .filter_map(|path| extract_metadata(&path))
        .map(|song| song.artist)
        .collect();

    artists.sort();
    artists.dedup();

    Ok(HttpResponse::Ok().json(artists))
}

/// Get unique albums in the library.
///
/// GET /api/music/albums
#[get("/api/music/albums")]
pub async fn list_albums(
    _user: AuthenticatedUser,
    data: web::Data<AppState>,
) -> AppResult<HttpResponse> {
    let mut albums: Vec<String> = fs::read_dir(&data.music_folder)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_audio_file(path))
        .filter_map(|path| extract_metadata(&path))
        .map(|song| song.album)
        .collect();

    albums.sort();
    albums.dedup();

    Ok(HttpResponse::Ok().json(albums))
}

/// Configure music routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_music)
        .service(stream_music)
        .service(get_cover)
        .service(list_artists)
        .service(list_albums);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename_valid() {
        assert!(sanitize_filename("song.mp3").is_ok());
        assert!(sanitize_filename("My Song (2023).flac").is_ok());
    }

    #[test]
    fn test_sanitize_filename_path_traversal() {
        assert!(sanitize_filename("../etc/passwd").is_err());
        assert!(sanitize_filename("..\\windows\\system32").is_err());
        assert!(sanitize_filename("foo/../bar").is_err());
        assert!(sanitize_filename("/etc/passwd").is_err());
    }

    #[test]
    fn test_sanitize_filename_empty() {
        assert!(sanitize_filename("").is_err());
    }

    #[test]
    fn test_is_audio_file() {
        assert!(is_audio_file(Path::new("song.mp3")));
        assert!(is_audio_file(Path::new("song.FLAC")));
        assert!(!is_audio_file(Path::new("image.jpg")));
        assert!(!is_audio_file(Path::new("noextension")));
    }
}

