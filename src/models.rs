//! Data models for the application.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::auth::JsonUserRepository;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// Path to the music folder.
    pub music_folder: PathBuf,
    /// User repository.
    pub user_repo: std::sync::Arc<JsonUserRepository>,
}

/// Song metadata extracted from audio files.
#[derive(Debug, Clone, Serialize)]
pub struct SongMetadata {
    /// Unique identifier (hash of file path).
    pub id: String,
    /// Song title.
    pub title: String,
    /// Artist name.
    pub artist: String,
    /// Album name.
    pub album: String,
    /// Track duration in seconds.
    pub duration: Option<u32>,
    /// Track number in album.
    pub track_number: Option<u32>,
    /// Release year.
    pub year: Option<i32>,
    /// Genre.
    pub genre: Option<String>,
    /// Audio format (mp3, flac, etc.).
    pub format: String,
    /// Filename (used for streaming endpoint).
    pub file: String,
    /// Whether the track has embedded cover art.
    pub has_cover: bool,
}

impl SongMetadata {
    /// Generate a stable ID from file path.
    pub fn generate_id(path: &std::path::Path) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

/// Generic API response wrapper.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    /// Whether the request was successful.
    pub success: bool,
    /// Response data.
    pub data: T,
}

impl<T> ApiResponse<T> {
    /// Create a successful response.
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data,
        }
    }
}

/// Paginated response wrapper.
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    /// Items for the current page.
    pub items: Vec<T>,
    /// Total number of items.
    pub total: usize,
    /// Current page (1-indexed).
    pub page: usize,
    /// Items per page.
    pub per_page: usize,
    /// Total number of pages.
    pub total_pages: usize,
    /// Whether there is a next page.
    pub has_next: bool,
    /// Whether there is a previous page.
    pub has_prev: bool,
}

impl<T> PaginatedResponse<T> {
    /// Create a paginated response from a full collection.
    pub fn from_vec(items: Vec<T>, page: usize, per_page: usize, total: usize) -> Self {
        let total_pages = (total + per_page - 1) / per_page;

        Self {
            items,
            total,
            page,
            per_page,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}

/// Query parameters for listing songs.
#[derive(Debug, Deserialize)]
pub struct ListSongsQuery {
    /// Search query (searches title, artist, album).
    pub q: Option<String>,
    /// Filter by artist.
    pub artist: Option<String>,
    /// Filter by album.
    pub album: Option<String>,
    /// Filter by genre.
    pub genre: Option<String>,
    /// Page number (1-indexed).
    #[serde(default = "default_page")]
    pub page: usize,
    /// Items per page (max 100).
    #[serde(default = "default_per_page")]
    pub per_page: usize,
    /// Sort field.
    #[serde(default)]
    pub sort: SortField,
    /// Sort order.
    #[serde(default)]
    pub order: SortOrder,
}

fn default_page() -> usize {
    1
}

fn default_per_page() -> usize {
    50
}

/// Fields available for sorting.
#[derive(Debug, Default, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SortField {
    #[default]
    Title,
    Artist,
    Album,
    Year,
    Duration,
}

/// Sort order.
#[derive(Debug, Default, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paginated_response() {
        let items: Vec<i32> = vec![1, 2, 3];
        let response = PaginatedResponse::from_vec(items, 1, 10, 25);

        assert_eq!(response.total, 25);
        assert_eq!(response.total_pages, 3);
        assert!(response.has_next);
        assert!(!response.has_prev);
    }

    #[test]
    fn test_song_id_generation() {
        let path1 = std::path::Path::new("/music/song.mp3");
        let path2 = std::path::Path::new("/music/song.mp3");
        let path3 = std::path::Path::new("/music/other.mp3");

        let id1 = SongMetadata::generate_id(path1);
        let id2 = SongMetadata::generate_id(path2);
        let id3 = SongMetadata::generate_id(path3);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }
}
