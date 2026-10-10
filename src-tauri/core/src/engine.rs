//! The engine facade: everything a client (the Tauri app, a future CLI or server)
//! needs to embed Nsembe Play's media engine.

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::database::{folders, home, Db};
use crate::errors::Result;
use crate::events::EventSink;
use crate::library::Library;
use crate::models::{AppStatus, EngineKind, EngineSelection, ToolStatus};
use crate::player::PlayerManager;
use crate::scanner::Scanner;
use crate::settings::{self, Settings};
use crate::thumbnails::{ArtworkGenerator, ArtworkStore};
use crate::tools::{self, ToolOverrides, ToolPaths};

#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Where the database, artwork cache and mpv config live.
    pub data_dir: PathBuf,
    /// Directories that may contain bundled mpv/ffmpeg/ffprobe binaries.
    pub bundled_tool_dirs: Vec<PathBuf>,
    pub version: String,
}

pub struct MediaEngine {
    pub db: Db,
    pub settings: Settings,
    pub library: Arc<Library>,
    pub player: Arc<PlayerManager>,
    pub artwork: Arc<ArtworkGenerator>,
    tools: Arc<RwLock<ToolPaths>>,
    config: EngineConfig,
}

impl MediaEngine {
    pub async fn start(config: EngineConfig, sink: Arc<dyn EventSink>) -> Result<Arc<Self>> {
        std::fs::create_dir_all(&config.data_dir)?;
        let db = Db::open(&config.data_dir.join("library.db")).await?;
        let settings = Settings::new(db.clone());
        let tools = Arc::new(RwLock::new(tools::locate_all(&Self::overrides(&settings).await, &config.bundled_tool_dirs)));
        let store = ArtworkStore::new(config.data_dir.join("artwork"))?;
        let artwork = Arc::new(ArtworkGenerator::new(db.clone(), store, tools.clone(), sink.clone()));
        let scanner = Scanner::start(db.clone(), sink.clone(), tools.clone(), artwork.clone());
        let library = Library::start(db.clone(), sink.clone(), scanner).await?;
        let player = PlayerManager::new(db.clone(), sink, tools.clone(), config.data_dir.clone());
        Ok(Arc::new(MediaEngine { db, settings, library, player, artwork, tools, config }))
    }

    async fn overrides(settings: &Settings) -> ToolOverrides {
        let get = |v: String| if v.trim().is_empty() { None } else { Some(v) };
        ToolOverrides {
            mpv: get(settings.get_string(settings::MPV_PATH).await),
            ffmpeg: get(settings.get_string(settings::FFMPEG_PATH).await),
            ffprobe: get(settings.get_string(settings::FFPROBE_PATH).await),
        }
    }

    pub fn tools(&self) -> ToolPaths {
        self.tools.read().unwrap().clone()
    }

    pub fn artwork_dir(&self) -> PathBuf {
        self.artwork.store().root().to_path_buf()
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.config.data_dir
    }

    /// Re-detects external tools (after the user installs them or changes paths).
    pub async fn refresh_tools(&self) -> ToolStatus {
        let before = self.tools();
        let found = tools::locate_all(&Self::overrides(&self.settings).await, &self.config.bundled_tool_dirs);
        let status = found.status();
        let ffmpeg_appeared = before.ffmpeg.is_none() && found.ffmpeg.is_some();
        *self.tools.write().unwrap() = found;
        if ffmpeg_appeared {
            self.library.regenerate_artwork();
        }
        status
    }

    /// Which engine plays audio/video, resolving "auto" against available tools.
    pub async fn engines(&self) -> EngineSelection {
        let has_mpv = self.tools().mpv.is_some();
        let resolve = |pref: String| {
            if pref == "builtin" || !has_mpv {
                "builtin".to_string()
            } else {
                "mpv".to_string()
            }
        };
        EngineSelection {
            audio: resolve(self.settings.get_string(settings::AUDIO_ENGINE).await),
            video: resolve(self.settings.get_string(settings::VIDEO_ENGINE).await),
        }
    }

    pub async fn status(&self) -> Result<AppStatus> {
        Ok(AppStatus {
            version: self.config.version.clone(),
            onboarding_completed: self.settings.get_bool(settings::ONBOARDING_COMPLETED).await,
            data_dir: self.config.data_dir.to_string_lossy().into_owned(),
            tools: self.tools().status(),
            engines: self.engines().await,
            folder_count: folders::count(&self.db).await?,
            stats: home::stats(&self.db).await?,
        })
    }

    pub async fn shutdown(&self) {
        self.player.shutdown().await;
        self.db.close().await;
    }

    pub fn player_state(&self, engine: EngineKind) -> crate::models::PlayerState {
        self.player.state(engine)
    }
}
