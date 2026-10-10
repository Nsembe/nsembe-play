//! Data transfer types returned to clients. All serialize as camelCase JSON.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::errors::{CoreError, Result};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum FolderKind {
    Music,
    Movies,
    Tv,
}

impl FolderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            FolderKind::Music => "music",
            FolderKind::Movies => "movies",
            FolderKind::Tv => "tv",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "music" => Ok(FolderKind::Music),
            "movies" => Ok(FolderKind::Movies),
            "tv" => Ok(FolderKind::Tv),
            other => Err(CoreError::invalid(format!("Unknown library type '{other}'"))),
        }
    }

    /// The kind of media row produced for files in a folder of this kind.
    pub fn media_kind(&self) -> MediaKind {
        match self {
            FolderKind::Music => MediaKind::Track,
            FolderKind::Movies => MediaKind::Movie,
            FolderKind::Tv => MediaKind::Episode,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Track,
    Movie,
    Episode,
}

impl MediaKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaKind::Track => "track",
            MediaKind::Movie => "movie",
            MediaKind::Episode => "episode",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "track" => Some(MediaKind::Track),
            "movie" => Some(MediaKind::Movie),
            "episode" => Some(MediaKind::Episode),
            _ => None,
        }
    }

    pub fn is_video(&self) -> bool {
        !matches!(self, MediaKind::Track)
    }
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct LibraryFolder {
    pub id: i64,
    pub path: String,
    pub kind: String,
    pub enabled: bool,
    pub added_at: i64,
    pub last_scanned_at: Option<i64>,
    pub last_error: Option<String>,
    pub item_count: i64,
    pub missing_count: i64,
    pub error_count: i64,
    #[sqlx(skip)]
    pub accessible: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: i64,
    pub media_id: i64,
    pub title: String,
    pub artist_id: Option<i64>,
    pub artist_name: Option<String>,
    pub album_id: Option<i64>,
    pub album_title: Option<String>,
    pub genre: Option<String>,
    pub year: Option<i64>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
    pub duration: Option<f64>,
    pub artwork: Option<String>,
    pub added_at: i64,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: i64,
    pub title: String,
    pub artist_id: i64,
    pub artist_name: String,
    pub year: Option<i64>,
    pub genre: Option<String>,
    pub artwork: Option<String>,
    pub artwork_full: Option<String>,
    pub track_count: i64,
    pub duration: f64,
    pub added_at: i64,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: i64,
    pub name: String,
    pub artwork: Option<String>,
    pub album_count: i64,
    pub track_count: i64,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumDetail {
    pub album: Album,
    pub tracks: Vec<Track>,
    pub more_by_artist: Vec<Album>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistDetail {
    pub artist: Artist,
    pub albums: Vec<Album>,
    pub top_tracks: Vec<Track>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Genre {
    pub name: String,
    pub track_count: i64,
    pub artwork: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Movie {
    pub id: i64,
    pub media_id: i64,
    pub title: String,
    pub year: Option<i64>,
    pub genres: Option<String>,
    pub overview: Option<String>,
    pub duration: Option<f64>,
    pub poster: Option<String>,
    pub poster_full: Option<String>,
    pub backdrop: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub container: Option<String>,
    pub added_at: i64,
    pub position: Option<f64>,
    pub completed: Option<bool>,
    pub last_played_at: Option<i64>,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StreamInfo {
    pub index: i64,
    pub codec: Option<String>,
    pub language: Option<String>,
    pub title: Option<String>,
    pub channels: Option<i64>,
    pub default: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovieDetail {
    pub movie: Movie,
    pub path: String,
    pub file_size: i64,
    pub audio_streams: Vec<StreamInfo>,
    pub subtitle_streams: Vec<StreamInfo>,
    pub more_like_this: Vec<Movie>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Show {
    pub id: i64,
    pub title: String,
    pub year: Option<i64>,
    pub overview: Option<String>,
    pub genres: Option<String>,
    pub poster: Option<String>,
    pub poster_full: Option<String>,
    pub backdrop: Option<String>,
    pub season_count: i64,
    pub episode_count: i64,
    pub watched_count: i64,
    pub added_at: i64,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Season {
    pub id: i64,
    pub show_id: i64,
    pub season_number: i64,
    pub poster: Option<String>,
    pub episode_count: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub id: i64,
    pub media_id: i64,
    pub show_id: i64,
    pub show_title: String,
    pub season_number: i64,
    pub episode_number: i64,
    pub title: String,
    pub overview: Option<String>,
    pub duration: Option<f64>,
    pub thumb: Option<String>,
    pub position: Option<f64>,
    pub completed: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowDetail {
    pub show: Show,
    pub seasons: Vec<Season>,
    pub episodes: Vec<Episode>,
    /// The episode the "Play" button should start: in-progress, next unwatched or first.
    pub next_up: Option<Episode>,
}

/// A universal, playable representation of any media item (queue entries, history,
/// playlists, continue watching).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PlayableItem {
    pub media_id: i64,
    pub kind: String,
    pub path: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub artwork: Option<String>,
    pub backdrop: Option<String>,
    pub duration: Option<f64>,
    pub track_id: Option<i64>,
    pub album_id: Option<i64>,
    pub album_title: Option<String>,
    pub artist_id: Option<i64>,
    pub movie_id: Option<i64>,
    pub episode_id: Option<i64>,
    pub show_id: Option<i64>,
    pub position: Option<f64>,
    pub completed: Option<bool>,
    pub last_played_at: Option<i64>,
    pub missing: bool,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueItem {
    #[serde(flatten)]
    pub item: PlayableItem,
    /// "resume" for a partially watched item, "next" for the next episode of a show.
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct RecentItem {
    /// "album", "movie" or "tv_show".
    pub kind: String,
    pub id: i64,
    pub title: String,
    pub subtitle: Option<String>,
    pub artwork: Option<String>,
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub artwork: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub item_count: i64,
    pub duration: f64,
    /// Up to four artworks for a mosaic cover when no custom artwork exists.
    pub cover_artworks: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistEntry {
    /// playlist_items.id - used for remove/reorder.
    pub item_id: i64,
    pub position: i64,
    pub added_at: i64,
    pub media: PlayableItem,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDetail {
    pub playlist: Playlist,
    pub entries: Vec<PlaylistEntry>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStats {
    pub tracks: i64,
    pub albums: i64,
    pub artists: i64,
    pub movies: i64,
    pub shows: i64,
    pub episodes: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeFeed {
    pub continue_watching: Vec<ContinueItem>,
    pub recently_added: Vec<RecentItem>,
    pub recently_played: Vec<PlayableItem>,
    pub playlists: Vec<Playlist>,
    pub albums: Vec<Album>,
    pub movies: Vec<Movie>,
    pub shows: Vec<Show>,
    pub stats: LibraryStats,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FavoritesOverview {
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
    pub movies: Vec<Movie>,
    pub shows: Vec<Show>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub query: String,
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
    pub movies: Vec<Movie>,
    pub shows: Vec<Show>,
    pub playlists: Vec<Playlist>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub folder_id: Option<i64>,
    pub folder_path: Option<String>,
    /// "discovering", "indexing", "artwork", "idle"
    pub phase: String,
    pub processed: u64,
    pub total: u64,
    pub current_path: Option<String>,
    pub added: u64,
    pub updated: u64,
    pub removed: u64,
    pub errors: u64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    pub id: i64,
    /// "audio" or "sub"
    pub kind: String,
    pub title: Option<String>,
    pub language: Option<String>,
    pub codec: Option<String>,
    pub selected: bool,
    pub external: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum EngineKind {
    Audio,
    Video,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub engine: EngineKind,
    pub item: Option<PlayableItem>,
    pub position: f64,
    pub duration: Option<f64>,
    pub paused: bool,
    pub buffering: bool,
    pub volume: f64,
    pub muted: bool,
    pub speed: f64,
    pub fullscreen: bool,
    pub audio_tracks: Vec<TrackInfo>,
    pub subtitle_tracks: Vec<TrackInfo>,
    /// True while an mpv process is running for this engine.
    pub active: bool,
}

impl PlayerState {
    pub fn idle(engine: EngineKind) -> Self {
        PlayerState {
            engine,
            item: None,
            position: 0.0,
            duration: None,
            paused: true,
            buffering: false,
            volume: 80.0,
            muted: false,
            speed: 1.0,
            fullscreen: false,
            audio_tracks: vec![],
            subtitle_tracks: vec![],
            active: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerEnded {
    pub engine: EngineKind,
    pub media_id: i64,
    /// "eof", "error", "closed", "stopped"
    pub reason: String,
    /// The media that was started automatically afterwards (next episode), if any.
    pub next_media_id: Option<i64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub mpv: Option<String>,
    pub ffmpeg: Option<String>,
    pub ffprobe: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineSelection {
    /// "mpv" or "builtin"
    pub audio: String,
    pub video: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub version: String,
    pub onboarding_completed: bool,
    pub data_dir: String,
    pub tools: ToolStatus,
    pub engines: EngineSelection,
    pub folder_count: i64,
    pub stats: LibraryStats,
}

/// Sidecar subtitles, converted to WebVTT for the built-in player.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleFile {
    pub label: String,
    pub language: Option<String>,
    pub vtt: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProblemFile {
    pub media_id: i64,
    pub path: String,
    pub kind: String,
    pub error: Option<String>,
    pub missing: bool,
}
