//! Filesystem helpers: media type detection, directory discovery, content
//! fingerprints and folder watching.

pub mod watcher;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::models::FolderKind;
use crate::util::{extension_lower, mtime_of};

pub const AUDIO_EXTENSIONS: &[&str] =
    &["mp3", "flac", "m4a", "aac", "ogg", "oga", "opus", "wav", "wma", "aiff", "aif", "ape", "wv", "mpc", "alac", "dsf"];

pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "m4v", "avi", "mov", "wmv", "webm", "ts", "m2ts", "mts", "mpg", "mpeg", "flv", "3gp", "ogv", "divx",
];

/// Directories never worth descending into.
pub fn is_ignored_dir(name: &str) -> bool {
    name.starts_with('.')
        || name.starts_with('$')
        || matches!(
            name.to_ascii_lowercase().as_str(),
            "system volume information" | "@eadir" | "#recycle" | "lost+found" | "node_modules" | "sample" | "samples"
        )
}

/// Whether a file belongs in a library folder of the given kind.
pub fn is_media_for(kind: FolderKind, path: &Path) -> bool {
    let Some(ext) = extension_lower(path) else { return false };
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if file_name.starts_with('.') || file_name.starts_with("._") {
        return false;
    }
    match kind {
        FolderKind::Music => AUDIO_EXTENSIONS.contains(&ext.as_str()),
        FolderKind::Movies | FolderKind::Tv => VIDEO_EXTENSIONS.contains(&ext.as_str()),
    }
}

/// Small "sample" clips shipped alongside releases are not real movies/episodes.
pub fn is_sample(path: &Path, size: u64) -> bool {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    let looks_like_sample = stem == "sample" || stem.ends_with("-sample") || stem.ends_with(".sample") || stem.starts_with("sample-");
    looks_like_sample && size < 300 * 1024 * 1024
}

#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub path: PathBuf,
    pub size: i64,
    pub mtime: i64,
}

/// Stats a single candidate file, filtering out non-media and junk.
pub fn inspect_file(kind: FolderKind, path: &Path) -> Option<DiscoveredFile> {
    if !is_media_for(kind, path) {
        return None;
    }
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() == 0 {
        return None;
    }
    if kind != FolderKind::Music && is_sample(path, meta.len()) {
        return None;
    }
    Some(DiscoveredFile { path: path.to_path_buf(), size: meta.len() as i64, mtime: mtime_of(&meta) })
}

/// Recursively lists media files below `root`. Unreadable entries are skipped
/// (and logged) rather than failing the whole scan.
pub fn discover(root: &Path, kind: FolderKind) -> Vec<DiscoveredFile> {
    let mut out = Vec::new();
    let walker = WalkDir::new(root).follow_links(true).into_iter().filter_entry(|e| {
        e.depth() == 0 || !e.file_type().is_dir() || !is_ignored_dir(&e.file_name().to_string_lossy())
    });
    for entry in walker {
        match entry {
            Ok(e) if e.file_type().is_file() => {
                if let Some(f) = inspect_file(kind, e.path()) {
                    out.push(f);
                }
            }
            Ok(_) => {}
            Err(err) => log::warn!("Skipping unreadable entry: {err}"),
        }
    }
    out
}

const FINGERPRINT_CHUNK: u64 = 64 * 1024;

/// Cheap content fingerprint: size + hash of the first and last 64 KiB. Used to
/// recognise files that were moved or renamed while the app was not watching.
pub fn fingerprint(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    hasher.update(&size.to_le_bytes());
    let mut buf = vec![0u8; FINGERPRINT_CHUNK as usize];
    let n = file.read(&mut buf)?;
    hasher.update(&buf[..n]);
    if size > FINGERPRINT_CHUNK * 2 {
        file.seek(SeekFrom::End(-(FINGERPRINT_CHUNK as i64)))?;
        let n = file.read(&mut buf)?;
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:016x}", hasher.digest()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_detection() {
        assert!(is_media_for(FolderKind::Music, Path::new("a/b/Song.FLAC")));
        assert!(!is_media_for(FolderKind::Music, Path::new("a/b/movie.mkv")));
        assert!(is_media_for(FolderKind::Movies, Path::new("Inception (2010).mkv")));
        assert!(!is_media_for(FolderKind::Movies, Path::new("._Inception.mkv")));
        assert!(is_sample(Path::new("x/sample.mkv"), 1000));
        assert!(!is_sample(Path::new("x/Samples of Life.mkv"), 1000));
        assert!(is_ignored_dir("$RECYCLE.BIN"));
    }

    #[test]
    fn fingerprint_is_stable_and_content_based() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.bin");
        let b = dir.path().join("b.bin");
        std::fs::write(&a, vec![7u8; 300_000]).unwrap();
        std::fs::write(&b, vec![7u8; 300_000]).unwrap();
        assert_eq!(fingerprint(&a).unwrap(), fingerprint(&b).unwrap());
        std::fs::write(&b, vec![8u8; 300_000]).unwrap();
        assert_ne!(fingerprint(&a).unwrap(), fingerprint(&b).unwrap());
    }
}
