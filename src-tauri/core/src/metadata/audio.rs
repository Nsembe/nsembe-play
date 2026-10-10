//! Embedded audio metadata (ID3, Vorbis comments, MP4 atoms, ...) via `lofty`.

use std::path::Path;

use lofty::config::ParseOptions;
use lofty::picture::PictureType;
use lofty::prelude::*;
use lofty::probe::Probe;

#[derive(Debug, Clone, Default)]
pub struct AudioMeta {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<i64>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
    pub duration: Option<f64>,
    pub bitrate: Option<i64>,
    pub codec: Option<String>,
}

fn non_empty(s: Option<std::borrow::Cow<'_, str>>) -> Option<String> {
    s.map(|v| v.trim().trim_matches('\0').to_string()).filter(|v| !v.is_empty())
}

/// ID3 genres are sometimes stored as "(17)" or "17"; drop the numeric form.
fn clean_genre(g: String) -> Option<String> {
    let g = g.trim().trim_start_matches('(').trim_end_matches(')').to_string();
    if g.is_empty() || g.chars().all(|c| c.is_ascii_digit()) {
        None
    } else {
        Some(g)
    }
}

/// Reads tags and audio properties. Artwork is not decoded here (see [`read_cover`]).
pub fn read_audio_meta(path: &Path) -> Result<AudioMeta, String> {
    let mut options = ParseOptions::new();
    options.read_cover_art(false);
    let tagged = Probe::open(path)
        .map_err(|e| e.to_string())?
        .options(options)
        .read()
        .map_err(|e| format!("Could not read tags: {e}"))?;
    let props = tagged.properties();
    let duration = props.duration().as_secs_f64();
    let mut meta = AudioMeta {
        duration: if duration > 0.0 { Some(duration) } else { None },
        bitrate: props.audio_bitrate().map(|b| b as i64 * 1000),
        codec: Some(format!("{:?}", tagged.file_type()).to_lowercase()),
        ..Default::default()
    };
    if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
        meta.title = non_empty(tag.title());
        meta.artist = non_empty(tag.artist());
        meta.album = non_empty(tag.album());
        meta.genre = non_empty(tag.genre()).and_then(clean_genre);
        meta.year = tag.year().map(|y| y as i64).filter(|y| *y > 0);
        meta.track_number = tag.track().map(|n| n as i64);
        meta.disc_number = tag.disk().map(|n| n as i64);
        meta.album_artist =
            tag.get_string(&ItemKey::AlbumArtist).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        if meta.year.is_none() {
            meta.year = tag
                .get_string(&ItemKey::RecordingDate)
                .or_else(|| tag.get_string(&ItemKey::OriginalReleaseDate))
                .and_then(|d| d.get(..4))
                .and_then(|y| y.parse().ok());
        }
    }
    Ok(meta)
}

/// Extracts the embedded cover (front cover preferred) as raw image bytes.
pub fn read_cover(path: &Path) -> Option<Vec<u8>> {
    let tagged = lofty::read_from_path(path).ok()?;
    let mut best: Option<Vec<u8>> = None;
    for tag in tagged.tags() {
        for pic in tag.pictures() {
            if pic.pic_type() == PictureType::CoverFront {
                return Some(pic.data().to_vec());
            }
            if best.is_none() {
                best = Some(pic.data().to_vec());
            }
        }
    }
    best
}
