//! Tauri command handlers. These are thin adapters: validation and business
//! logic live in `nsembe-core`.

pub mod app;
pub mod browse;
pub mod library;
pub mod player;
pub mod playlists;

use std::sync::Arc;

use nsembe_core::MediaEngine;
use tauri::State;

use crate::AppState;

pub(crate) fn engine(state: &State<'_, AppState>) -> Arc<MediaEngine> {
    state.engine.clone()
}
