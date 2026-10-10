use serde::Serialize;

use crate::models::{PlayerEnded, PlayerState, ScanProgress};

/// Notifications emitted by the engine. Clients forward them to their UI.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum CoreEvent {
    /// Progress of a running scan.
    ScanProgress(ScanProgress),
    /// The library contents changed (scan finished, watcher update, folder removed).
    LibraryChanged { reason: String },
    /// New artwork/thumbnails are available.
    ArtworkUpdated,
    /// Playback state of an engine changed.
    PlayerState(Box<PlayerState>),
    /// A media item finished, failed or its window was closed.
    PlayerEnded(PlayerEnded),
    /// A user-facing message (warnings, errors that happened in the background).
    Notice(Notice),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub level: NoticeLevel,
    pub title: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NoticeLevel {
    Info,
    Warning,
    Error,
}

impl Notice {
    pub fn error(title: impl Into<String>, message: impl Into<String>) -> CoreEvent {
        CoreEvent::Notice(Notice { level: NoticeLevel::Error, title: title.into(), message: message.into() })
    }
    pub fn warning(title: impl Into<String>, message: impl Into<String>) -> CoreEvent {
        CoreEvent::Notice(Notice { level: NoticeLevel::Warning, title: title.into(), message: message.into() })
    }
}

/// Receives engine events. Implementations must be cheap and non-blocking.
pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: CoreEvent);
}

/// Sink that discards everything (tests, headless tools).
pub struct NoopSink;

impl EventSink for NoopSink {
    fn emit(&self, _event: CoreEvent) {}
}
