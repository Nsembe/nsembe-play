use nsembe_core::database::playlists;
use nsembe_core::models::{Playlist, PlaylistDetail};
use tauri::State;

use super::engine;
use crate::error::CmdResult;
use crate::AppState;

#[tauri::command]
pub async fn playlists_list(state: State<'_, AppState>) -> CmdResult<Vec<Playlist>> {
    Ok(playlists::list(&engine(&state).db).await?)
}

#[tauri::command]
pub async fn playlist_detail(state: State<'_, AppState>, id: i64) -> CmdResult<PlaylistDetail> {
    Ok(playlists::detail(&engine(&state).db, id).await?)
}

#[tauri::command]
pub async fn playlist_create(state: State<'_, AppState>, name: String, description: Option<String>) -> CmdResult<Playlist> {
    Ok(playlists::create(&engine(&state).db, &name, description.as_deref()).await?)
}

#[tauri::command]
pub async fn playlist_update(
    state: State<'_, AppState>,
    id: i64,
    name: String,
    description: Option<String>,
) -> CmdResult<Playlist> {
    Ok(playlists::update(&engine(&state).db, id, &name, description.as_deref()).await?)
}

#[tauri::command]
pub async fn playlist_delete(state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    Ok(playlists::delete(&engine(&state).db, id).await?)
}

#[tauri::command]
pub async fn playlist_add_items(state: State<'_, AppState>, id: i64, media_ids: Vec<i64>) -> CmdResult<u64> {
    Ok(playlists::add_items(&engine(&state).db, id, &media_ids).await?)
}

#[tauri::command]
pub async fn playlist_remove_item(state: State<'_, AppState>, item_id: i64) -> CmdResult<()> {
    Ok(playlists::remove_item(&engine(&state).db, item_id).await?)
}

#[tauri::command]
pub async fn playlist_reorder(state: State<'_, AppState>, id: i64, item_ids: Vec<i64>) -> CmdResult<()> {
    Ok(playlists::reorder(&engine(&state).db, id, &item_ids).await?)
}
