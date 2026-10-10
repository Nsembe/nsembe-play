use std::path::Path;

use nsembe_core::database::folders;
use nsembe_core::models::{FolderKind, LibraryFolder, ProblemFile, ScanProgress};
use tauri::{AppHandle, State};

use super::engine;
use crate::error::CmdResult;
use crate::{allow_asset_dir, AppState};

#[tauri::command]
pub async fn library_folders(state: State<'_, AppState>) -> CmdResult<Vec<LibraryFolder>> {
    Ok(engine(&state).library.folders().await?)
}

#[tauri::command]
pub async fn library_add_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    kind: FolderKind,
) -> CmdResult<LibraryFolder> {
    let folder = engine(&state).library.add_folder(&path, kind).await?;
    allow_asset_dir(&app, Path::new(&folder.path));
    Ok(folder)
}

#[tauri::command]
pub async fn library_remove_folder(state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    Ok(engine(&state).library.remove_folder(id).await?)
}

#[tauri::command]
pub async fn library_rescan(state: State<'_, AppState>, folder_id: Option<i64>, force: Option<bool>) -> CmdResult<()> {
    Ok(engine(&state).library.rescan(folder_id, force.unwrap_or(false)).await?)
}

#[tauri::command]
pub fn library_scan_status(state: State<'_, AppState>) -> ScanProgress {
    state.engine.library.scan_progress()
}

#[tauri::command]
pub async fn library_cleanup_missing(state: State<'_, AppState>) -> CmdResult<u64> {
    Ok(engine(&state).library.cleanup_missing().await?)
}

#[tauri::command]
pub async fn library_problem_files(state: State<'_, AppState>) -> CmdResult<Vec<ProblemFile>> {
    Ok(folders::problem_files(&engine(&state).db, 200).await?)
}
