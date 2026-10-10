//! Sidecar subtitle discovery (`Movie.en.srt`, `Movie.vtt`) and SRT -> WebVTT
//! conversion for the built-in HTML player. mpv loads sidecar files itself.

use std::path::Path;

use crate::models::SubtitleFile;

const SUB_EXTENSIONS: [&str; 2] = ["srt", "vtt"];

pub fn srt_to_vtt(srt: &str) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for line in srt.trim_start_matches('\u{feff}').lines() {
        if line.contains("-->") {
            out.push_str(&line.replace(',', "."));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

/// Finds subtitle files that share the media file's stem in the same directory.
pub fn find_sidecar_subtitles(media_path: &Path) -> Vec<SubtitleFile> {
    let (Some(dir), Some(stem)) = (media_path.parent(), media_path.file_stem().and_then(|s| s.to_str())) else {
        return vec![];
    };
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    let stem_lower = stem.to_lowercase();
    let mut subs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(ext) = crate::util::extension_lower(&path) else { continue };
        if !SUB_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|s| s.to_str()) else { continue };
        if !name.to_lowercase().starts_with(&stem_lower) {
            continue;
        }
        // "Movie.en.forced" -> "en.forced"
        let suffix = name.get(stem.len()..).unwrap_or("").trim_start_matches(['.', '_', '-', ' ']).to_string();
        let Ok(bytes) = std::fs::read(&path) else { continue };
        if bytes.len() > 5 * 1024 * 1024 {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        let vtt = if ext == "srt" { srt_to_vtt(&text) } else { text.into_owned() };
        let language = suffix.split('.').next().filter(|s| (2..=3).contains(&s.len())).map(str::to_string);
        subs.push(SubtitleFile {
            label: if suffix.is_empty() { "Subtitles".to_string() } else { suffix.replace('.', " ") },
            language,
            vtt,
        });
    }
    subs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_srt_timestamps() {
        let srt = "1\n00:00:01,000 --> 00:00:02,500\nHello\n";
        let vtt = srt_to_vtt(srt);
        assert!(vtt.starts_with("WEBVTT"));
        assert!(vtt.contains("00:00:01.000 --> 00:00:02.500"));
        assert!(vtt.contains("Hello"));
    }
}
