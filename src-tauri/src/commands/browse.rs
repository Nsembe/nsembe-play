use nsembe_core::database::{favorites, home, music, playback, search as search_db, video};
use nsembe_core::models::{
    AlbumDetail, Album, Artist, ArtistDetail, ContinueItem, FavoritesOverview, Genre, HomeFeed, Movie, MovieDetail, Page,
    PlayableItem, RecentItem, SearchResults, Show, ShowDetail, Track,
};
use tauri::{AppHandle, Emitter, State};

use super::engine;
use crate::error::CmdResult;
use crate::AppState;

const DEFAULT_PAGE: i64 = 60;

#[tauri::command]
pub async fn home_feed(state: State<'_, AppState>) -> CmdResult<HomeFeed> {
    Ok(home::feed(&engine(&state).db).await?)
}

#[tauri::command]
pub async fn recently_added(state: State<'_, AppState>, offset: Option<i64>, limit: Option<i64>) -> CmdResult<Page<RecentItem>> {
    Ok(home::recently_added(&engine(&state).db, offset.unwrap_or(0), limit.unwrap_or(DEFAULT_PAGE)).await?)
}

#[tauri::command]
pub async fn recently_played(
    state: State<'_, AppState>,
    kind: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
) -> CmdResult<Page<PlayableItem>> {
    Ok(playback::recently_played(&engine(&state).db, kind.as_deref(), offset.unwrap_or(0), limit.unwrap_or(DEFAULT_PAGE))
        .await?)
}

#[tauri::command]
pub async fn continue_watching(state: State<'_, AppState>) -> CmdResult<Vec<ContinueItem>> {
    Ok(playback::continue_watching(&engine(&state).db, 30).await?)
}

#[tauri::command]
pub async fn movies_list(
    state: State<'_, AppState>,
    sort: Option<String>,
    genre: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
) -> CmdResult<Page<Movie>> {
    let genre = genre.filter(|g| !g.is_empty());
    Ok(video::list_movies(
        &engine(&state).db,
        sort.as_deref().unwrap_or("title"),
        genre.as_deref(),
        offset.unwrap_or(0),
        limit.unwrap_or(DEFAULT_PAGE),
    )
    .await?)
}

#[tauri::command]
pub async fn movie_genres(state: State<'_, AppState>) -> CmdResult<Vec<String>> {
    Ok(video::movie_genres(&engine(&state).db).await?)
}

#[tauri::command]
pub async fn movie_detail(state: State<'_, AppState>, id: i64) -> CmdResult<MovieDetail> {
    Ok(video::movie_detail(&engine(&state).db, id).await?)
}

#[tauri::command]
pub async fn shows_list(
    state: State<'_, AppState>,
    sort: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
) -> CmdResult<Page<Show>> {
    Ok(video::list_shows(&engine(&state).db, sort.as_deref().unwrap_or("title"), offset.unwrap_or(0), limit.unwrap_or(DEFAULT_PAGE))
        .await?)
}

#[tauri::command]
pub async fn show_detail(state: State<'_, AppState>, id: i64) -> CmdResult<ShowDetail> {
    Ok(video::show_detail(&engine(&state).db, id).await?)
}

#[tauri::command]
pub async fn tracks_list(
    state: State<'_, AppState>,
    sort: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
) -> CmdResult<Page<Track>> {
    Ok(music::list_tracks(&engine(&state).db, sort.as_deref().unwrap_or("title"), offset.unwrap_or(0), limit.unwrap_or(200))
        .await?)
}

#[tauri::command]
pub async fn genre_tracks(
    state: State<'_, AppState>,
    genre: String,
    offset: Option<i64>,
    limit: Option<i64>,
) -> CmdResult<Page<Track>> {
    Ok(music::genre_tracks(&engine(&state).db, &genre, offset.unwrap_or(0), limit.unwrap_or(200)).await?)
}

#[tauri::command]
pub async fn genres_list(state: State<'_, AppState>) -> CmdResult<Vec<Genre>> {
    Ok(music::genres(&engine(&state).db).await?)
}

#[tauri::command]
pub async fn albums_list(
    state: State<'_, AppState>,
    sort: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
) -> CmdResult<Page<Album>> {
    Ok(music::list_albums(&engine(&state).db, sort.as_deref().unwrap_or("title"), offset.unwrap_or(0), limit.unwrap_or(DEFAULT_PAGE))
        .await?)
}

#[tauri::command]
pub async fn album_detail(state: State<'_, AppState>, id: i64) -> CmdResult<AlbumDetail> {
    Ok(music::album_detail(&engine(&state).db, id).await?)
}

#[tauri::command]
pub async fn artists_list(state: State<'_, AppState>, offset: Option<i64>, limit: Option<i64>) -> CmdResult<Page<Artist>> {
    Ok(music::list_artists(&engine(&state).db, offset.unwrap_or(0), limit.unwrap_or(DEFAULT_PAGE)).await?)
}

#[tauri::command]
pub async fn artist_detail(state: State<'_, AppState>, id: i64) -> CmdResult<ArtistDetail> {
    Ok(music::artist_detail(&engine(&state).db, id).await?)
}

#[tauri::command]
pub async fn favorites_overview(state: State<'_, AppState>) -> CmdResult<FavoritesOverview> {
    Ok(favorites::overview(&engine(&state).db).await?)
}

#[tauri::command]
pub async fn favorite_toggle(state: State<'_, AppState>, item_type: String, item_id: i64) -> CmdResult<bool> {
    Ok(favorites::toggle(&engine(&state).db, &item_type, item_id).await?)
}

#[tauri::command]
pub async fn search(state: State<'_, AppState>, query: String, limit: Option<usize>) -> CmdResult<SearchResults> {
    Ok(search_db::search(&engine(&state).db, &query, limit.unwrap_or(8)).await?)
}

/// Resolves media ids to playable items in the given order (queues).
#[tauri::command]
pub async fn media_playables(state: State<'_, AppState>, media_ids: Vec<i64>) -> CmdResult<Vec<PlayableItem>> {
    Ok(playback::playables(&engine(&state).db, &media_ids).await?)
}

#[tauri::command]
pub async fn music_shuffle(state: State<'_, AppState>, limit: Option<i64>) -> CmdResult<Vec<PlayableItem>> {
    let db = &engine(&state).db;
    let ids = music::random_track_media_ids(db, limit.unwrap_or(200)).await?;
    Ok(playback::playables(db, &ids).await?)
}

#[tauri::command]
pub async fn history_clear(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    playback::clear_history(&engine(&state).db).await?;
    // Reuse the library-changed notification so every view refreshes.
    app.emit("library://changed", "history")?;
    Ok(())
}

#[tauri::command]
pub async fn media_set_watched(state: State<'_, AppState>, media_id: i64, watched: bool) -> CmdResult<()> {
    Ok(playback::set_watched(&engine(&state).db, media_id, watched).await?)
}
