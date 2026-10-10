use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Current unix time in seconds.
pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Modification time of a file in unix seconds (0 when unavailable).
pub fn mtime_of(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Lower-cased file extension without the dot.
pub fn extension_lower(path: &Path) -> Option<String> {
    path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase())
}

pub fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Returns true when `child` is `parent` or lives below it.
pub fn path_is_within(child: &Path, parent: &Path) -> bool {
    child.starts_with(parent)
}

/// Platform path separator as a string (used for prefix matching in SQL).
pub fn separator() -> &'static str {
    std::path::MAIN_SEPARATOR_STR
}

#[cfg(windows)]
pub fn hide_console(cmd: &mut tokio::process::Command) {
    // CREATE_NO_WINDOW: avoids flashing a console when spawning ffprobe/ffmpeg.
    cmd.creation_flags(0x0800_0000);
}

#[cfg(not(windows))]
pub fn hide_console(_cmd: &mut tokio::process::Command) {}
