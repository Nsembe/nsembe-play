//! Artwork and thumbnail generation.
//!
//! Artwork is produced in a background pass after indexing so scanning is never
//! blocked by image work. The pass is resumable: rows are flagged with
//! `artwork_checked` once handled, so an interrupted pass continues next time.
//! Generated images live in the app's artwork cache - original media folders are
//! never written to.

use std::io::{BufWriter, Cursor};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use futures::StreamExt;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageReader};
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::database::Db;
use crate::events::{CoreEvent, EventSink};
use crate::metadata::{audio, nfo};
use crate::settings::{self, Settings};
use crate::tools::ToolPaths;
use crate::util::{hide_console, now};

const FULL_MAX: u32 = 1280;
const THUMB_MAX: u32 = 360;
const IMAGE_EXTENSIONS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];

/// Location of generated artwork.
#[derive(Clone)]
pub struct ArtworkStore {
    root: PathBuf,
}

impl ArtworkStore {
    pub fn new(root: PathBuf) -> std::io::Result<Self> {
        for sub in ["albums", "movies", "shows", "seasons", "frames"] {
            std::fs::create_dir_all(root.join(sub))?;
        }
        Ok(ArtworkStore { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Unique file name per generation so the webview never shows a cached old image.
    fn file(&self, dir: &str, id: i64, suffix: &str) -> PathBuf {
        self.root.join(dir).join(format!("{id}{suffix}-{}.jpg", now()))
    }
}

fn decode_bytes(bytes: &[u8]) -> Result<DynamicImage, String> {
    ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| format!("Unsupported or corrupted image: {e}"))
}

fn decode_file(path: &Path) -> Result<DynamicImage, String> {
    ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| format!("Unsupported or corrupted image: {e}"))
}

/// Resizes (never upscales) and writes a JPEG.
pub fn save_resized(img: &DynamicImage, dest: &Path, max: u32) -> Result<(), String> {
    let resized = if img.width() > max || img.height() > max { img.resize(max, max, FilterType::CatmullRom) } else { img.clone() };
    let rgb = resized.to_rgb8();
    let file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);
    JpegEncoder::new_with_quality(&mut writer, 86).encode_image(&rgb).map_err(|e| e.to_string())
}

fn remove_old(path: Option<&str>) {
    if let Some(p) = path {
        let _ = std::fs::remove_file(p);
    }
}

/// First existing image named `<name>.<ext>` (case-insensitive) in `dir`.
pub fn find_image(dir: &Path, names: &[String]) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let files: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
    for name in names {
        let name = name.to_lowercase();
        for f in &files {
            let stem = f.file_stem().and_then(|s| s.to_str()).map(str::to_lowercase);
            let ext = crate::util::extension_lower(f);
            if stem.as_deref() == Some(name.as_str()) && ext.as_deref().is_some_and(|e| IMAGE_EXTENSIONS.contains(&e)) {
                return Some(f.clone());
            }
        }
    }
    None
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// Extracts a single frame with ffmpeg.
pub async fn grab_frame(ffmpeg: &Path, video: &Path, at_seconds: f64, dest: &Path) -> Result<(), String> {
    let mut cmd = Command::new(ffmpeg);
    cmd.args(["-hide_banner", "-loglevel", "error", "-ss"])
        .arg(format!("{:.2}", at_seconds.max(0.0)))
        .arg("-i")
        .arg(video)
        .args(["-frames:v", "1", "-vf", "scale='min(1280,iw)':-2", "-q:v", "3", "-y"])
        .arg(dest)
        .kill_on_drop(true)
        .stdin(std::process::Stdio::null());
    hide_console(&mut cmd);
    let output = tokio::time::timeout(Duration::from_secs(60), cmd.output())
        .await
        .map_err(|_| "ffmpeg timed out".to_string())?
        .map_err(|e| format!("Could not run ffmpeg: {e}"))?;
    if !output.status.success() || !dest.is_file() {
        return Err(format!("Frame extraction failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }
    Ok(())
}

fn frame_time(duration: Option<f64>, fraction: f64) -> f64 {
    match duration {
        Some(d) if d > 10.0 => d * fraction,
        Some(d) => d / 2.0,
        None => 60.0,
    }
}

/// Generates missing artwork in the background.
pub struct ArtworkGenerator {
    db: Db,
    store: ArtworkStore,
    tools: Arc<RwLock<ToolPaths>>,
    settings: Settings,
    sink: Arc<dyn EventSink>,
    lock: Mutex<()>,
}

impl ArtworkGenerator {
    pub fn new(db: Db, store: ArtworkStore, tools: Arc<RwLock<ToolPaths>>, sink: Arc<dyn EventSink>) -> Self {
        let settings = Settings::new(db.clone());
        ArtworkGenerator { db, store, tools, settings, sink, lock: Mutex::new(()) }
    }

    pub fn store(&self) -> &ArtworkStore {
        &self.store
    }

    fn ffmpeg(&self) -> Option<PathBuf> {
        self.tools.read().ok().and_then(|t| t.ffmpeg.clone())
    }

    /// Processes everything pending. Concurrent calls are serialized.
    pub async fn process_pending(&self) {
        let _guard = self.lock.lock().await;
        if let Err(e) = self.albums().await {
            log::error!("Album artwork pass failed: {e}");
        }
        if let Err(e) = self.shows().await {
            log::error!("TV artwork pass failed: {e}");
        }
        if let Err(e) = self.movies().await {
            log::error!("Movie artwork pass failed: {e}");
        }
        if let Err(e) = self.episodes().await {
            log::error!("Episode thumbnail pass failed: {e}");
        }
    }

    async fn albums(&self) -> crate::Result<()> {
        let mut cursor = 0i64;
        loop {
            let rows = sqlx::query_as::<_, (i64, Option<String>, Option<String>, Option<String>)>(
                "SELECT a.id, a.artwork_path, a.artwork_thumb,
                        (SELECT m.path FROM tracks t JOIN media m ON m.id = t.media_id
                         WHERE t.album_id = a.id AND m.missing = 0
                         ORDER BY t.disc_number, t.track_number LIMIT 1)
                 FROM albums a WHERE a.artwork_checked = 0 AND a.id > ? ORDER BY a.id LIMIT 40",
            )
            .bind(cursor)
            .fetch_all(self.db.pool())
            .await?;
            if rows.is_empty() {
                return Ok(());
            }
            let mut changed = false;
            for (id, old_full, old_thumb, track_path) in rows {
                cursor = id;
                let Some(track_path) = track_path else {
                    sqlx::query("UPDATE albums SET artwork_checked = 1 WHERE id = ?").bind(id).execute(self.db.pool()).await?;
                    continue;
                };
                let full = self.store.file("albums", id, "");
                let thumb = self.store.file("albums", id, "_t");
                let (f2, t2) = (full.clone(), thumb.clone());
                let result = tokio::task::spawn_blocking(move || -> Result<bool, String> {
                    let path = PathBuf::from(&track_path);
                    let img = match audio::read_cover(&path) {
                        Some(bytes) => Some(decode_bytes(&bytes)?),
                        None => {
                            let dir = path.parent().ok_or("no parent")?;
                            match find_image(dir, &names(&["cover", "folder", "front", "album", "albumart", "albumartsmall"])) {
                                Some(p) => Some(decode_file(&p)?),
                                None => None,
                            }
                        }
                    };
                    match img {
                        Some(img) => {
                            save_resized(&img, &f2, 1000)?;
                            save_resized(&img, &t2, THUMB_MAX)?;
                            Ok(true)
                        }
                        None => Ok(false),
                    }
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()));
                match result {
                    Ok(true) => {
                        remove_old(old_full.as_deref());
                        remove_old(old_thumb.as_deref());
                        sqlx::query("UPDATE albums SET artwork_path = ?, artwork_thumb = ?, artwork_checked = 1 WHERE id = ?")
                            .bind(full.to_string_lossy().as_ref())
                            .bind(thumb.to_string_lossy().as_ref())
                            .bind(id)
                            .execute(self.db.pool())
                            .await?;
                        changed = true;
                    }
                    Ok(false) => {
                        sqlx::query("UPDATE albums SET artwork_checked = 1 WHERE id = ?").bind(id).execute(self.db.pool()).await?;
                    }
                    Err(e) => {
                        log::warn!("Album artwork for {id} failed: {e}");
                        sqlx::query("UPDATE albums SET artwork_checked = 1 WHERE id = ?").bind(id).execute(self.db.pool()).await?;
                    }
                }
            }
            if changed {
                self.sink.emit(CoreEvent::ArtworkUpdated);
            }
        }
    }

    async fn shows(&self) -> crate::Result<()> {
        let rows = sqlx::query_as::<_, (i64, Option<String>, Option<String>, Option<String>)>(
            "SELECT id, folder_path, poster_path, poster_thumb FROM tv_shows WHERE artwork_checked = 0",
        )
        .fetch_all(self.db.pool())
        .await?;
        let mut changed = false;
        for (id, folder, old_poster, old_thumb) in rows {
            if let Some(dir) = folder.as_deref().map(PathBuf::from).filter(|d| d.is_dir()) {
                // tvshow.nfo: overview, genres, year
                if let Some(info) = nfo::find_show_nfo(&dir) {
                    sqlx::query(
                        "UPDATE tv_shows SET overview = COALESCE(?, overview), year = COALESCE(year, ?),
                            genres = COALESCE(?, genres) WHERE id = ?",
                    )
                    .bind(info.plot)
                    .bind(info.year)
                    .bind(if info.genres.is_empty() { None } else { Some(info.genres.join(",")) })
                    .bind(id)
                    .execute(self.db.pool())
                    .await?;
                    changed = true;
                }
                let poster = find_image(&dir, &names(&["poster", "folder", "cover", "show"]));
                let backdrop = find_image(&dir, &names(&["fanart", "backdrop", "background", "banner"]));
                let full = self.store.file("shows", id, "");
                let thumb = self.store.file("shows", id, "_t");
                let back = self.store.file("shows", id, "_b");
                let (f2, t2, b2) = (full.clone(), thumb.clone(), back.clone());
                let (p2, bd2) = (poster.clone(), backdrop.clone());
                let res = tokio::task::spawn_blocking(move || -> Result<(bool, bool), String> {
                    let mut out = (false, false);
                    if let Some(p) = p2 {
                        let img = decode_file(&p)?;
                        save_resized(&img, &f2, 1000)?;
                        save_resized(&img, &t2, THUMB_MAX + 120)?;
                        out.0 = true;
                    }
                    if let Some(b) = bd2 {
                        save_resized(&decode_file(&b)?, &b2, FULL_MAX)?;
                        out.1 = true;
                    }
                    Ok(out)
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()));
                match res {
                    Ok((has_poster, has_backdrop)) => {
                        if has_poster {
                            remove_old(old_poster.as_deref());
                            remove_old(old_thumb.as_deref());
                            sqlx::query("UPDATE tv_shows SET poster_path = ?, poster_thumb = ? WHERE id = ?")
                                .bind(full.to_string_lossy().as_ref())
                                .bind(thumb.to_string_lossy().as_ref())
                                .bind(id)
                                .execute(self.db.pool())
                                .await?;
                            changed = true;
                        }
                        if has_backdrop {
                            sqlx::query("UPDATE tv_shows SET backdrop_path = ? WHERE id = ?")
                                .bind(back.to_string_lossy().as_ref())
                                .bind(id)
                                .execute(self.db.pool())
                                .await?;
                            changed = true;
                        }
                    }
                    Err(e) => log::warn!("Show artwork for {id} failed: {e}"),
                }
                changed |= self.season_posters(id, &dir).await?;
            }
            sqlx::query("UPDATE tv_shows SET artwork_checked = 1 WHERE id = ?").bind(id).execute(self.db.pool()).await?;
        }
        if changed {
            self.sink.emit(CoreEvent::ArtworkUpdated);
        }
        Ok(())
    }

    async fn season_posters(&self, show_id: i64, show_dir: &Path) -> crate::Result<bool> {
        let seasons = sqlx::query_as::<_, (i64, i64, Option<String>)>(
            "SELECT id, season_number, folder_path FROM tv_seasons WHERE show_id = ? AND poster_path IS NULL",
        )
        .bind(show_id)
        .fetch_all(self.db.pool())
        .await?;
        let mut changed = false;
        for (season_id, number, folder) in seasons {
            let mut candidates = vec![
                format!("season{number:02}-poster"),
                format!("season{number}-poster"),
                format!("season{number:02}"),
                format!("season {number:02}"),
            ];
            if number == 0 {
                candidates.insert(0, "season-specials-poster".into());
            }
            let mut found = find_image(show_dir, &candidates);
            if found.is_none() {
                if let Some(dir) = folder.as_deref().map(PathBuf::from) {
                    found = find_image(&dir, &names(&["poster", "folder", "cover"]));
                }
            }
            let Some(src) = found else { continue };
            let dest = self.store.file("seasons", season_id, "");
            let d2 = dest.clone();
            let ok = tokio::task::spawn_blocking(move || decode_file(&src).and_then(|img| save_resized(&img, &d2, THUMB_MAX + 120)))
                .await
                .map(|r| r.is_ok())
                .unwrap_or(false);
            if ok {
                sqlx::query("UPDATE tv_seasons SET poster_path = ? WHERE id = ?")
                    .bind(dest.to_string_lossy().as_ref())
                    .bind(season_id)
                    .execute(self.db.pool())
                    .await?;
                changed = true;
            }
        }
        Ok(changed)
    }

    async fn movies(&self) -> crate::Result<()> {
        let mut cursor = 0i64;
        loop {
            let rows = sqlx::query_as::<_, MovieArtRow>(
                "SELECT mv.id, m.id AS media_id, m.path, m.duration, f.path AS root,
                        mv.poster_path, mv.poster_thumb, mv.backdrop_path, m.thumb_path
                 FROM movies mv JOIN media m ON m.id = mv.media_id JOIN library_folders f ON f.id = m.folder_id
                 WHERE m.artwork_checked = 0 AND m.missing = 0 AND mv.id > ? ORDER BY mv.id LIMIT 12",
            )
            .bind(cursor)
            .fetch_all(self.db.pool())
            .await?;
            if rows.is_empty() {
                return Ok(());
            }
            cursor = rows.last().map(|r| r.id).unwrap_or(cursor);
            let results: Vec<crate::Result<bool>> =
                futures::stream::iter(rows).map(|row| self.movie_artwork(row)).buffer_unordered(3).collect().await;
            if results.iter().any(|r| matches!(r, Ok(true))) {
                self.sink.emit(CoreEvent::ArtworkUpdated);
            }
            for r in results {
                if let Err(e) = r {
                    log::warn!("Movie artwork failed: {e}");
                }
            }
        }
    }

    async fn movie_artwork(&self, row: MovieArtRow) -> crate::Result<bool> {
        let path = PathBuf::from(&row.path);
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
        let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        let dedicated = dir != Path::new(&row.root);
        let mut poster_names = vec![format!("{stem}-poster"), stem.clone()];
        let mut backdrop_names = vec![format!("{stem}-fanart"), format!("{stem}-backdrop")];
        if dedicated {
            poster_names.extend(names(&["poster", "folder", "cover", "movie"]));
            backdrop_names.extend(names(&["fanart", "backdrop", "background"]));
        }
        let poster_src = find_image(&dir, &poster_names);
        let backdrop_src = find_image(&dir, &backdrop_names);

        let full = self.store.file("movies", row.id, "");
        let thumb = self.store.file("movies", row.id, "_t");
        let back = self.store.file("movies", row.id, "_b");
        let frame = self.store.file("frames", row.media_id, "");
        let mut changed = false;

        let (f2, t2, b2) = (full.clone(), thumb.clone(), back.clone());
        let (ps, bs) = (poster_src.clone(), backdrop_src.clone());
        let local = tokio::task::spawn_blocking(move || -> (bool, bool) {
            let poster = ps
                .map(|p| decode_file(&p).and_then(|img| save_resized(&img, &f2, 1000).and_then(|_| save_resized(&img, &t2, THUMB_MAX + 120))))
                .map(|r| r.is_ok())
                .unwrap_or(false);
            let backdrop = bs.map(|b| decode_file(&b).and_then(|img| save_resized(&img, &b2, FULL_MAX)).is_ok()).unwrap_or(false);
            (poster, backdrop)
        })
        .await
        .unwrap_or((false, false));

        if local.0 {
            remove_old(row.poster_path.as_deref());
            remove_old(row.poster_thumb.as_deref());
            sqlx::query("UPDATE movies SET poster_path = ?, poster_thumb = ? WHERE id = ?")
                .bind(full.to_string_lossy().as_ref())
                .bind(thumb.to_string_lossy().as_ref())
                .bind(row.id)
                .execute(self.db.pool())
                .await?;
            changed = true;
        }
        if local.1 {
            remove_old(row.backdrop_path.as_deref());
            sqlx::query("UPDATE movies SET backdrop_path = ? WHERE id = ?")
                .bind(back.to_string_lossy().as_ref())
                .bind(row.id)
                .execute(self.db.pool())
                .await?;
            changed = true;
        }

        let ffmpeg = self.ffmpeg();
        let thumbnails_enabled = self.settings.get_bool(settings::VIDEO_THUMBNAILS).await;
        let mut attempted_frame = false;
        if let (Some(ffmpeg), true) = (ffmpeg, thumbnails_enabled) {
            attempted_frame = true;
            match grab_frame(&ffmpeg, &path, frame_time(row.duration, 0.2), &frame).await {
                Ok(()) => {
                    remove_old(row.thumb_path.as_deref());
                    sqlx::query("UPDATE media SET thumb_path = ? WHERE id = ?")
                        .bind(frame.to_string_lossy().as_ref())
                        .bind(row.media_id)
                        .execute(self.db.pool())
                        .await?;
                    changed = true;
                }
                Err(e) => log::warn!("Thumbnail for {} failed: {e}", row.path),
            }
        }
        let done = attempted_frame || (local.0 && local.1) || !thumbnails_enabled;
        if done {
            sqlx::query("UPDATE media SET artwork_checked = 1 WHERE id = ?").bind(row.media_id).execute(self.db.pool()).await?;
        }
        Ok(changed)
    }

    async fn episodes(&self) -> crate::Result<()> {
        let Some(ffmpeg) = self.ffmpeg() else { return Ok(()) };
        if !self.settings.get_bool(settings::VIDEO_THUMBNAILS).await {
            return Ok(());
        }
        let mut cursor = 0i64;
        loop {
            let rows = sqlx::query_as::<_, (i64, String, Option<f64>, Option<String>)>(
                "SELECT m.id, m.path, m.duration, m.thumb_path FROM media m
                 WHERE m.kind = 'episode' AND m.artwork_checked = 0 AND m.missing = 0 AND m.id > ?
                 ORDER BY m.id LIMIT 12",
            )
            .bind(cursor)
            .fetch_all(self.db.pool())
            .await?;
            if rows.is_empty() {
                return Ok(());
            }
            cursor = rows.last().map(|r| r.0).unwrap_or(cursor);
            let results: Vec<crate::Result<bool>> = futures::stream::iter(rows)
                .map(|(media_id, path, duration, old)| {
                    let ffmpeg = ffmpeg.clone();
                    async move {
                        let dest = self.store.file("frames", media_id, "");
                        let ok = match grab_frame(&ffmpeg, Path::new(&path), frame_time(duration, 0.25), &dest).await {
                            Ok(()) => {
                                remove_old(old.as_deref());
                                sqlx::query("UPDATE media SET thumb_path = ?, artwork_checked = 1 WHERE id = ?")
                                    .bind(dest.to_string_lossy().as_ref())
                                    .bind(media_id)
                                    .execute(self.db.pool())
                                    .await?;
                                true
                            }
                            Err(e) => {
                                log::warn!("Thumbnail for {path} failed: {e}");
                                sqlx::query("UPDATE media SET artwork_checked = 1 WHERE id = ?")
                                    .bind(media_id)
                                    .execute(self.db.pool())
                                    .await?;
                                false
                            }
                        };
                        Ok(ok)
                    }
                })
                .buffer_unordered(3)
                .collect()
                .await;
            if results.iter().any(|r| matches!(r, Ok(true))) {
                self.sink.emit(CoreEvent::ArtworkUpdated);
            }
        }
    }
}

#[derive(sqlx::FromRow)]
struct MovieArtRow {
    id: i64,
    media_id: i64,
    path: String,
    duration: Option<f64>,
    root: String,
    poster_path: Option<String>,
    poster_thumb: Option<String>,
    backdrop_path: Option<String>,
    thumb_path: Option<String>,
}
