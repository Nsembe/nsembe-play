//! Nsembe Play desktop shell: wires the UI-agnostic media engine (`nsembe-core`)
//! to the webview through Tauri commands and events.

mod commands;
mod error;
mod events;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use nsembe_core::{EngineConfig, MediaEngine};
use tauri::{AppHandle, Manager, RunEvent};

pub struct AppState {
    pub engine: Arc<MediaEngine>,
}

/// Lets the webview read media files of a library folder through the asset
/// protocol (used by the built-in player). Nothing outside library folders and
/// the artwork cache is reachable.
pub(crate) fn allow_asset_dir(app: &AppHandle, dir: &Path) {
    if let Err(e) = app.asset_protocol_scope().allow_directory(dir, true) {
        log::warn!("Could not allow asset access to {}: {e}", dir.display());
    }
}

fn bundled_tool_dirs(app: &AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(res) = app.path().resource_dir() {
        dirs.push(res.join("bin"));
        dirs.push(res.join("resources").join("bin"));
    }
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        dirs.push(exe_dir.join("bin"));
        dirs.push(exe_dir);
    }
    // Development builds: tools downloaded into src-tauri/resources/bin.
    #[cfg(debug_assertions)]
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources").join("bin"));
    dirs
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .clear_targets()
                .level(log::LevelFilter::Info)
                .level_for("sqlx", log::LevelFilter::Warn)
                .level_for("tao", log::LevelFilter::Warn)
                .target(tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout))
                .target(tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                    file_name: Some("nsembe-play".into()),
                }))
                .max_file_size(5_000_000)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let data_dir = app.path().app_local_data_dir()?;
            let config = EngineConfig {
                data_dir,
                bundled_tool_dirs: bundled_tool_dirs(&handle),
                version: app.package_info().version.to_string(),
            };
            let sink = Arc::new(events::TauriSink(handle.clone()));
            let engine = tauri::async_runtime::block_on(MediaEngine::start(config, sink))
                .map_err(|e| format!("Could not start the media engine: {e}"))?;

            allow_asset_dir(&handle, &engine.artwork_dir());
            if let Ok(folders) = tauri::async_runtime::block_on(engine.library.folders()) {
                for f in folders {
                    allow_asset_dir(&handle, Path::new(&f.path));
                }
            }
            app.manage(AppState { engine });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_status,
            commands::app::app_complete_onboarding,
            commands::app::settings_get,
            commands::app::settings_set,
            commands::app::tools_refresh,
            commands::app::media_reveal,
            commands::library::library_folders,
            commands::library::library_add_folder,
            commands::library::library_remove_folder,
            commands::library::library_rescan,
            commands::library::library_scan_status,
            commands::library::library_cleanup_missing,
            commands::library::library_problem_files,
            commands::browse::home_feed,
            commands::browse::recently_added,
            commands::browse::recently_played,
            commands::browse::continue_watching,
            commands::browse::movies_list,
            commands::browse::movie_genres,
            commands::browse::movie_detail,
            commands::browse::shows_list,
            commands::browse::show_detail,
            commands::browse::tracks_list,
            commands::browse::genre_tracks,
            commands::browse::genres_list,
            commands::browse::albums_list,
            commands::browse::album_detail,
            commands::browse::artists_list,
            commands::browse::artist_detail,
            commands::browse::favorites_overview,
            commands::browse::favorite_toggle,
            commands::browse::search,
            commands::browse::media_playables,
            commands::browse::music_shuffle,
            commands::browse::history_clear,
            commands::browse::media_set_watched,
            commands::playlists::playlists_list,
            commands::playlists::playlist_detail,
            commands::playlists::playlist_create,
            commands::playlists::playlist_update,
            commands::playlists::playlist_delete,
            commands::playlists::playlist_add_items,
            commands::playlists::playlist_remove_item,
            commands::playlists::playlist_reorder,
            commands::player::player_play,
            commands::player::player_command,
            commands::player::player_state,
            commands::player::playback_resolve,
            commands::player::playback_report,
            commands::player::player_adjacent_episode,
            commands::player::media_subtitles,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Nsembe Play");

    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            // Persist playback positions and close mpv before exiting.
            if let Some(state) = handle.try_state::<AppState>() {
                let engine = state.engine.clone();
                tauri::async_runtime::block_on(engine.shutdown());
            }
        }
    });
}
