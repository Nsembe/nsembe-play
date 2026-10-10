//! Typed application settings stored as JSON values in the `settings` table.

use serde_json::{json, Map, Value};

use crate::database::{settings_store, Db};
use crate::errors::{CoreError, Result};

pub const ONBOARDING_COMPLETED: &str = "app.onboardingCompleted";
/// "auto" | "mpv" | "builtin"
pub const AUDIO_ENGINE: &str = "player.audioEngine";
/// "auto" | "mpv" | "builtin"
pub const VIDEO_ENGINE: &str = "player.videoEngine";
pub const AUTOPLAY_NEXT_EPISODE: &str = "player.autoplayNextEpisode";
pub const VIDEO_FULLSCREEN: &str = "player.videoFullscreen";
pub const VOLUME: &str = "player.volume";
pub const AUDIO_LANGUAGE: &str = "player.audioLanguage";
pub const SUBTITLE_LANGUAGE: &str = "player.subtitleLanguage";
pub const MPV_PATH: &str = "tools.mpvPath";
pub const FFMPEG_PATH: &str = "tools.ffmpegPath";
pub const FFPROBE_PATH: &str = "tools.ffprobePath";
pub const WATCH_FOLDERS: &str = "library.watchFolders";
pub const VIDEO_THUMBNAILS: &str = "library.videoThumbnails";

fn defaults() -> Map<String, Value> {
    let mut m = Map::new();
    m.insert(ONBOARDING_COMPLETED.into(), json!(false));
    m.insert(AUDIO_ENGINE.into(), json!("auto"));
    m.insert(VIDEO_ENGINE.into(), json!("auto"));
    m.insert(AUTOPLAY_NEXT_EPISODE.into(), json!(true));
    m.insert(VIDEO_FULLSCREEN.into(), json!(false));
    m.insert(VOLUME.into(), json!(80));
    m.insert(AUDIO_LANGUAGE.into(), json!(""));
    m.insert(SUBTITLE_LANGUAGE.into(), json!(""));
    m.insert(MPV_PATH.into(), json!(""));
    m.insert(FFMPEG_PATH.into(), json!(""));
    m.insert(FFPROBE_PATH.into(), json!(""));
    m.insert(WATCH_FOLDERS.into(), json!(true));
    m.insert(VIDEO_THUMBNAILS.into(), json!(true));
    m
}

#[derive(Clone)]
pub struct Settings {
    db: Db,
}

impl Settings {
    pub fn new(db: Db) -> Self {
        Settings { db }
    }

    /// All settings with defaults applied.
    pub async fn all(&self) -> Result<Map<String, Value>> {
        let mut out = defaults();
        for (k, v) in settings_store::all(&self.db).await? {
            if let Ok(value) = serde_json::from_str::<Value>(&v) {
                out.insert(k, value);
            }
        }
        Ok(out)
    }

    pub async fn get(&self, key: &str) -> Result<Value> {
        match settings_store::get(&self.db, key).await? {
            Some(v) => Ok(serde_json::from_str(&v).unwrap_or(Value::Null)),
            None => Ok(defaults().get(key).cloned().unwrap_or(Value::Null)),
        }
    }

    pub async fn get_bool(&self, key: &str) -> bool {
        self.get(key).await.ok().and_then(|v| v.as_bool()).unwrap_or(false)
    }

    pub async fn get_string(&self, key: &str) -> String {
        self.get(key).await.ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
    }

    pub async fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key).await.ok().and_then(|v| v.as_f64())
    }

    /// Stores a setting. Only known keys are accepted, with type checking against the default.
    pub async fn set(&self, key: &str, value: Value) -> Result<()> {
        let defaults = defaults();
        let Some(default) = defaults.get(key) else {
            return Err(CoreError::invalid(format!("Unknown setting '{key}'")));
        };
        let type_ok = matches!(
            (default, &value),
            (Value::Bool(_), Value::Bool(_)) | (Value::String(_), Value::String(_)) | (Value::Number(_), Value::Number(_))
        );
        if !type_ok {
            return Err(CoreError::invalid(format!("Invalid value for setting '{key}'")));
        }
        if key == AUDIO_ENGINE || key == VIDEO_ENGINE {
            let v = value.as_str().unwrap_or_default();
            if !["auto", "mpv", "builtin"].contains(&v) {
                return Err(CoreError::invalid("Engine must be auto, mpv or builtin"));
            }
        }
        settings_store::set(&self.db, key, &value.to_string()).await
    }
}
