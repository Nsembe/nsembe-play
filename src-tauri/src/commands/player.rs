use std::path::Path;

use nsembe_core::database::playback;
use nsembe_core::metadata::subtitles;
use nsembe_core::models::{EngineKind, PlayableItem, PlayerState, SubtitleFile};
use nsembe_core::player::{ExternalEvent, PlayOptions, PlayerCommand};
use nsembe_core::CoreError;
use tauri::{AppHandle, Manager, State};

use super::engine;
use crate::error::CmdResult;
use crate::AppState;

/// mpv window geometry matching the main window, so video opens "inside" the app.
fn main_window_geometry(app: &AppHandle) -> Option<String> {
    let window = app.get_webview_window("main")?;
    if window.is_minimized().unwrap_or(false) {
        return None;
    }
    let pos = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    Some(format!("{}x{}+{}+{}", size.width, size.height, pos.x.max(0), pos.y.max(0)))
}

/// Plays a media item with mpv (audio engine for music, video window for video).
#[tauri::command]
pub async fn player_play(
    app: AppHandle,
    state: State<'_, AppState>,
    media_id: i64,
    start: Option<f64>,
) -> CmdResult<PlayableItem> {
    let options = PlayOptions { start, geometry: main_window_geometry(&app) };
    Ok(engine(&state).player.play(media_id, options).await?)
}

#[tauri::command]
pub async fn player_command(state: State<'_, AppState>, engine_kind: EngineKind, command: PlayerCommand) -> CmdResult<()> {
    Ok(engine(&state).player.command(engine_kind, command).await?)
}

#[tauri::command]
pub fn player_state(state: State<'_, AppState>, engine_kind: EngineKind) -> PlayerState {
    state.engine.player.state(engine_kind)
}

/// Item details plus resume position for the built-in (webview) player.
#[tauri::command]
pub async fn playback_resolve(state: State<'_, AppState>, media_id: i64) -> CmdResult<PlayableItem> {
    let db = &engine(&state).db;
    let mut item = playback::playable(db, media_id).await?;
    if !Path::new(&item.path).is_file() {
        nsembe_core::database::media::set_missing(db, media_id).await?;
        return Err(CoreError::MissingFile(format!(
            "\"{}\" could not be found. It may have been moved, renamed or deleted.",
            item.title
        ))
        .into());
    }
    item.position = playback::resume_position(db, media_id).await?;
    Ok(item)
}

/// Progress reports from the built-in player. Returns whether the item is completed.
#[tauri::command]
pub async fn playback_report(
    state: State<'_, AppState>,
    media_id: i64,
    event: ExternalEvent,
    position: f64,
    duration: Option<f64>,
) -> CmdResult<bool> {
    Ok(engine(&state).player.report_external(media_id, event, position, duration).await?)
}

#[tauri::command]
pub async fn player_adjacent_episode(
    state: State<'_, AppState>,
    media_id: i64,
    forward: bool,
) -> CmdResult<Option<PlayableItem>> {
    Ok(engine(&state).player.adjacent_episode(media_id, forward).await?)
}

/// Sidecar subtitles converted to WebVTT (built-in player).
#[tauri::command]
pub async fn media_subtitles(state: State<'_, AppState>, media_id: i64) -> CmdResult<Vec<SubtitleFile>> {
    let item = playback::playable(&engine(&state).db, media_id).await?;
    let path = item.path.clone();
    Ok(tokio::task::spawn_blocking(move || subtitles::find_sidecar_subtitles(Path::new(&path)))
        .await
        .map_err(|e| CoreError::Other(e.to_string()))?)
}
