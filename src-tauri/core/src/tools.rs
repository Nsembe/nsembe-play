//! Discovery of external media tools (mpv, ffmpeg, ffprobe).
//!
//! Lookup order: explicit path from settings -> bundled `bin` directories shipped
//! with the app -> `PATH` -> well-known install locations. Every tool is optional:
//! the engine degrades gracefully when one is missing.

use std::path::{Path, PathBuf};

use crate::models::ToolStatus;

#[derive(Debug, Clone, Default)]
pub struct ToolPaths {
    pub mpv: Option<PathBuf>,
    pub ffmpeg: Option<PathBuf>,
    pub ffprobe: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct ToolOverrides {
    pub mpv: Option<String>,
    pub ffmpeg: Option<String>,
    pub ffprobe: Option<String>,
}

impl ToolPaths {
    pub fn status(&self) -> ToolStatus {
        let s = |p: &Option<PathBuf>| p.as_ref().map(|p| p.to_string_lossy().into_owned());
        ToolStatus { mpv: s(&self.mpv), ffmpeg: s(&self.ffmpeg), ffprobe: s(&self.ffprobe) }
    }
}

fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn well_known_dirs(name: &str) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if cfg!(windows) {
        if let Ok(pf) = std::env::var("ProgramFiles") {
            dirs.push(Path::new(&pf).join(name));
            dirs.push(Path::new(&pf).join(name).join("bin"));
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            dirs.push(Path::new(&local).join("Microsoft").join("WinGet").join("Links"));
            dirs.push(Path::new(&local).join("Programs").join(name));
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            dirs.push(Path::new(&home).join("scoop").join("shims"));
        }
        dirs.push(PathBuf::from(r"C:\ProgramData\chocolatey\bin"));
        dirs.push(PathBuf::from(format!(r"C:\{name}")));
        dirs.push(PathBuf::from(format!(r"C:\{name}\bin")));
    } else {
        dirs.push(PathBuf::from("/usr/local/bin"));
        dirs.push(PathBuf::from("/opt/homebrew/bin"));
        dirs.push(PathBuf::from("/usr/bin"));
    }
    dirs
}

/// Finds a single tool.
pub fn locate(name: &str, override_path: Option<&str>, bundled_dirs: &[PathBuf]) -> Option<PathBuf> {
    if let Some(p) = override_path.map(str::trim).filter(|p| !p.is_empty()) {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
        log::warn!("Configured {name} path {} does not exist", p.display());
    }
    let exe = exe_name(name);
    for dir in bundled_dirs {
        let candidate = dir.join(&exe);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    if let Ok(p) = which::which(name) {
        return Some(p);
    }
    well_known_dirs(name).into_iter().map(|d| d.join(&exe)).find(|p| p.is_file())
}

pub fn locate_all(overrides: &ToolOverrides, bundled_dirs: &[PathBuf]) -> ToolPaths {
    let paths = ToolPaths {
        mpv: locate("mpv", overrides.mpv.as_deref(), bundled_dirs),
        ffmpeg: locate("ffmpeg", overrides.ffmpeg.as_deref(), bundled_dirs),
        ffprobe: locate("ffprobe", overrides.ffprobe.as_deref(), bundled_dirs),
    };
    log::info!("Media tools: mpv={:?} ffmpeg={:?} ffprobe={:?}", paths.mpv, paths.ffmpeg, paths.ffprobe);
    paths
}
