use std::path::PathBuf;
use serde::Serialize;

#[derive(Clone)]
pub struct AppState {
    pub music_folder: PathBuf,
}

#[derive(Serialize)]
pub struct SongMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: Option<u32>,
    pub file: String,
}
