use nsembe_core::{CoreEvent, EventSink};
use tauri::{AppHandle, Emitter};

/// Forwards engine events to the webview.
pub struct TauriSink(pub AppHandle);

impl EventSink for TauriSink {
    fn emit(&self, event: CoreEvent) {
        let result = match &event {
            CoreEvent::ScanProgress(p) => self.0.emit("library://scan-progress", p),
            CoreEvent::LibraryChanged { reason } => self.0.emit("library://changed", reason),
            CoreEvent::ArtworkUpdated => self.0.emit("library://artwork", ()),
            CoreEvent::PlayerState(s) => self.0.emit("player://state", s),
            CoreEvent::PlayerEnded(e) => self.0.emit("player://ended", e),
            CoreEvent::Notice(n) => self.0.emit("app://notice", n),
        };
        if let Err(e) = result {
            log::warn!("Could not emit event: {e}");
        }
    }
}
