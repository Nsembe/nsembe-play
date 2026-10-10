use nsembe_core::database::playback;
use nsembe_core::models::{AppStatus, ToolStatus};
use nsembe_core::settings;
use serde_json::{json, Map, Value};
use tauri::State;

use super::engine;
use crate::error::CmdResult;
use crate::AppState;

#[tauri::command]
pub async fn app_status(state: State<'_, AppState>) -> CmdResult<AppStatus> {
    Ok(engine(&state).status().await?)
}

#[tauri::command]
pub async fn app_complete_onboarding(state: State<'_, AppState>) -> CmdResult<()> {
    Ok(engine(&state).settings.set(settings::ONBOARDING_COMPLETED, json!(true)).await?)
}

#[tauri::command]
pub async fn settings_get(state: State<'_, AppState>) -> CmdResult<Map<String, Value>> {
    Ok(engine(&state).settings.all().await?)
}

#[tauri::command]
pub async fn settings_set(state: State<'_, AppState>, key: String, value: Value) -> CmdResult<Map<String, Value>> {
    let engine = engine(&state);
    engine.settings.set(&key, value.clone()).await?;
    match key.as_str() {
        settings::WATCH_FOLDERS => engine.library.set_watching(value.as_bool().unwrap_or(true)).await?,
        settings::MPV_PATH | settings::FFMPEG_PATH | settings::FFPROBE_PATH => {
            engine.refresh_tools().await;
        }
        settings::VIDEO_THUMBNAILS if value.as_bool() == Some(true) => engine.library.regenerate_artwork(),
        _ => {}
    }
    Ok(engine.settings.all().await?)
}

#[tauri::command]
pub async fn tools_refresh(state: State<'_, AppState>) -> CmdResult<ToolStatus> {
    Ok(engine(&state).refresh_tools().await)
}

/// Shows a media file in the system file manager.
#[tauri::command]
pub async fn media_reveal(state: State<'_, AppState>, media_id: i64) -> CmdResult<()> {
    let item = playback::playable(&engine(&state).db, media_id).await?;
    let path = std::path::PathBuf::from(&item.path);
    let result = if cfg!(windows) {
        std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn().map(|_| ())
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg("-R").arg(&path).spawn().map(|_| ())
    } else {
        let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or(path);
        std::process::Command::new("xdg-open").arg(dir).spawn().map(|_| ())
    };
    result.map_err(|e| nsembe_core::CoreError::Io(e).into())
}
