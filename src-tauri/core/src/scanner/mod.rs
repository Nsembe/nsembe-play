//! Library scanner.
//!
//! All indexing work runs on a single background worker so database writes are
//! serialized and the UI thread is never blocked. Full scans are incremental:
//! files whose size and modification time are unchanged are skipped.
//!
//! Pipeline per file: stat -> fingerprint -> metadata (lofty / ffprobe / .nfo /
//! filename) -> SQLite -> (later) artwork pass -> UI notification.

pub mod parse;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use futures::StreamExt;
use tokio::sync::mpsc;

use crate::database::media::{self as media_db, EpisodeRecord, FileRecord, MovieRecord, TechInfo, TrackRecord};
use crate::database::{folders, Db};
use crate::errors::Result;
use crate::events::{CoreEvent, EventSink, Notice};
use crate::filesystem::{self, DiscoveredFile};
use crate::metadata::{audio, nfo, video};
use crate::models::{FolderKind, LibraryFolder, MediaKind, ScanProgress};
use crate::thumbnails::ArtworkGenerator;
use crate::tools::ToolPaths;

/// Work items for the scanner worker.
#[derive(Debug, Clone)]
pub enum ScanJob {
    /// Walk a whole folder. `force` re-reads metadata even for unchanged files.
    Full { folder_id: i64, force: bool },
    /// Apply filesystem watcher changes for one folder.
    Changes { folder_id: i64, upserts: Vec<PathBuf>, removals: Vec<PathBuf>, renames: Vec<(PathBuf, PathBuf)>, attempt: u32 },
    /// Run the artwork pass only (e.g. after ffmpeg became available).
    Artwork,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ScanStats {
    pub added: u64,
    pub updated: u64,
    pub removed: u64,
    pub errors: u64,
}

impl ScanStats {
    fn changed(&self) -> bool {
        self.added + self.updated + self.removed > 0
    }
}

enum KindData {
    Track(TrackRecord),
    Movie(MovieRecord),
    Episode(EpisodeRecord),
}

struct Extracted {
    file: FileRecord,
    data: KindData,
}

#[derive(Clone)]
pub struct Scanner {
    tx: mpsc::UnboundedSender<ScanJob>,
    progress: Arc<Mutex<ScanProgress>>,
}

impl Scanner {
    pub fn start(
        db: Db,
        sink: Arc<dyn EventSink>,
        tools: Arc<RwLock<ToolPaths>>,
        artwork: Arc<ArtworkGenerator>,
    ) -> Scanner {
        let (tx, rx) = mpsc::unbounded_channel();
        let progress = Arc::new(Mutex::new(ScanProgress { phase: "idle".into(), ..Default::default() }));
        let worker = Worker { db, sink, tools, artwork, progress: progress.clone(), tx: tx.clone(), last_emit: Instant::now() };
        tokio::spawn(worker.run(rx));
        Scanner { tx, progress }
    }

    pub fn enqueue(&self, job: ScanJob) {
        if self.tx.send(job).is_err() {
            log::error!("Scanner worker is not running");
        }
    }

    pub fn progress(&self) -> ScanProgress {
        self.progress.lock().unwrap().clone()
    }
}

struct Worker {
    db: Db,
    sink: Arc<dyn EventSink>,
    tools: Arc<RwLock<ToolPaths>>,
    artwork: Arc<ArtworkGenerator>,
    progress: Arc<Mutex<ScanProgress>>,
    tx: mpsc::UnboundedSender<ScanJob>,
    last_emit: Instant,
}

impl Worker {
    async fn run(mut self, mut rx: mpsc::UnboundedReceiver<ScanJob>) {
        let mut library_dirty = false;
        let mut artwork_pending = false;
        while let Some(job) = rx.recv().await {
            let result = match job {
                ScanJob::Full { folder_id, force } => self.full_scan(folder_id, force).await,
                ScanJob::Changes { folder_id, upserts, removals, renames, attempt } => {
                    self.apply_changes(folder_id, upserts, removals, renames, attempt).await
                }
                ScanJob::Artwork => {
                    artwork_pending = true;
                    Ok(ScanStats::default())
                }
            };
            match result {
                Ok(stats) => {
                    if stats.changed() {
                        library_dirty = true;
                        artwork_pending = true;
                    }
                }
                Err(e) => {
                    log::error!("Scan job failed: {e}");
                    self.sink.emit(Notice::error("Library scan failed", e.to_string()));
                }
            }
            // Only finish up once the queue has drained, so bursts of jobs share one pass.
            if rx.is_empty() {
                if library_dirty {
                    if let Err(e) = media_db::prune_orphans(&self.db).await {
                        log::error!("Pruning orphans failed: {e}");
                    }
                    self.sink.emit(CoreEvent::LibraryChanged { reason: "scan".into() });
                    library_dirty = false;
                }
                if artwork_pending {
                    self.set_progress(|p| {
                        p.phase = "artwork".into();
                        p.active = true;
                        p.current_path = None;
                    });
                    self.artwork.process_pending().await;
                    artwork_pending = false;
                }
                self.set_progress(|p| {
                    *p = ScanProgress { phase: "idle".into(), ..Default::default() };
                });
            }
        }
    }

    fn set_progress(&mut self, f: impl FnOnce(&mut ScanProgress)) {
        let snapshot = {
            let mut p = self.progress.lock().unwrap();
            f(&mut p);
            p.clone()
        };
        self.sink.emit(CoreEvent::ScanProgress(snapshot));
        self.last_emit = Instant::now();
    }

    /// Like `set_progress` but rate limited (for per-file updates).
    fn tick_progress(&mut self, f: impl FnOnce(&mut ScanProgress)) {
        let snapshot = {
            let mut p = self.progress.lock().unwrap();
            f(&mut p);
            p.clone()
        };
        if self.last_emit.elapsed() >= Duration::from_millis(200) {
            self.sink.emit(CoreEvent::ScanProgress(snapshot));
            self.last_emit = Instant::now();
        }
    }

    fn ffprobe(&self) -> Option<PathBuf> {
        self.tools.read().ok().and_then(|t| t.ffprobe.clone())
    }

    async fn full_scan(&mut self, folder_id: i64, force: bool) -> Result<ScanStats> {
        let folder = match folders::get(&self.db, folder_id).await {
            Ok(f) => f,
            Err(_) => return Ok(ScanStats::default()), // removed meanwhile
        };
        let kind = FolderKind::parse(&folder.kind)?;
        let root = PathBuf::from(&folder.path);
        if !root.is_dir() {
            // Never mark files missing just because a drive is disconnected.
            let msg = format!("{} is not accessible. Is the drive connected?", folder.path);
            folders::set_error(&self.db, folder_id, Some(&msg)).await?;
            self.sink.emit(Notice::warning("Library folder unavailable", msg));
            return Ok(ScanStats::default());
        }
        log::info!("Scanning {} ({})", folder.path, folder.kind);
        self.set_progress(|p| {
            *p = ScanProgress {
                folder_id: Some(folder_id),
                folder_path: Some(folder.path.clone()),
                phase: "discovering".into(),
                active: true,
                ..Default::default()
            }
        });

        let root2 = root.clone();
        let files = tokio::task::spawn_blocking(move || filesystem::discover(&root2, kind))
            .await
            .map_err(|e| crate::CoreError::Other(e.to_string()))?;
        let existing = media_db::existing_for_folder(&self.db, folder_id).await?;

        let mut stats = ScanStats::default();
        let mut seen = HashSet::with_capacity(files.len());
        let mut to_process = Vec::new();
        for f in files {
            let key = f.path.to_string_lossy().into_owned();
            let unchanged = existing.get(&key).is_some_and(|e| e.size == f.size && e.mtime == f.mtime && !e.missing);
            seen.insert(key);
            if force || !unchanged {
                to_process.push(f);
            }
        }
        let gone: Vec<i64> = existing.iter().filter(|(p, e)| !e.missing && !seen.contains(*p)).map(|(_, e)| e.id).collect();
        if !gone.is_empty() {
            stats.removed += media_db::mark_missing_ids(&self.db, &gone).await?;
        }

        self.index_files(&folder, kind, &root, to_process, &mut stats).await;
        folders::mark_scanned(&self.db, folder_id, None).await?;
        log::info!(
            "Scan of {} done: {} added, {} updated, {} missing, {} errors",
            folder.path,
            stats.added,
            stats.updated,
            stats.removed,
            stats.errors
        );
        Ok(stats)
    }

    async fn apply_changes(
        &mut self,
        folder_id: i64,
        upserts: Vec<PathBuf>,
        removals: Vec<PathBuf>,
        renames: Vec<(PathBuf, PathBuf)>,
        attempt: u32,
    ) -> Result<ScanStats> {
        let Ok(folder) = folders::get(&self.db, folder_id).await else { return Ok(ScanStats::default()) };
        let kind = FolderKind::parse(&folder.kind)?;
        let root = PathBuf::from(&folder.path);
        let mut stats = ScanStats::default();
        let mut forced: Vec<PathBuf> = Vec::new();

        for (from, to) in renames {
            let moved = media_db::rename_under(&self.db, &from.to_string_lossy(), &to.to_string_lossy(), folder_id).await?;
            if moved.is_empty() {
                // Not known yet (e.g. renamed from a temp download name): index as new.
                forced.push(to);
            } else {
                stats.updated += moved.len() as u64;
                // Titles derive from file names, so re-read the renamed items.
                forced.extend(moved.into_iter().map(|(_, p)| PathBuf::from(p)));
            }
        }
        for path in removals {
            if !path.exists() {
                stats.removed += media_db::mark_missing_under(&self.db, &path.to_string_lossy()).await?;
            }
        }

        // Expand directories and drop non-media files.
        let mut candidates: Vec<(PathBuf, bool)> = Vec::new();
        for p in upserts {
            if p.is_dir() {
                let p2 = p.clone();
                let found = tokio::task::spawn_blocking(move || filesystem::discover(&p2, kind)).await.unwrap_or_default();
                candidates.extend(found.into_iter().map(|f| (f.path, false)));
            } else if filesystem::is_media_for(kind, &p) {
                candidates.push((p, false));
            }
        }
        candidates.extend(forced.into_iter().filter(|p| filesystem::is_media_for(kind, p)).map(|p| (p, true)));
        candidates.sort();
        candidates.dedup_by(|a, b| a.0 == b.0);

        // Files still being copied keep changing size: wait for them to settle.
        let first: Vec<Option<DiscoveredFile>> = candidates.iter().map(|(p, _)| filesystem::inspect_file(kind, p)).collect();
        if !candidates.is_empty() {
            tokio::time::sleep(Duration::from_millis(1200)).await;
        }
        let mut to_process = Vec::new();
        let mut unstable = Vec::new();
        for ((path, force), before) in candidates.into_iter().zip(first) {
            let Some(before) = before else { continue };
            let Some(now) = filesystem::inspect_file(kind, &path) else { continue };
            let readable = std::fs::File::open(&path).is_ok();
            if now.size != before.size || !readable {
                unstable.push(path);
                continue;
            }
            if !force {
                if let Some(existing) = media_db::find_by_path(&self.db, &now.path.to_string_lossy()).await? {
                    if existing.size == now.size && existing.mtime == now.mtime && !existing.missing {
                        continue;
                    }
                }
            }
            to_process.push(now);
        }
        if !unstable.is_empty() {
            self.retry_later(folder_id, unstable, attempt);
        }
        if !to_process.is_empty() {
            self.set_progress(|p| {
                *p = ScanProgress {
                    folder_id: Some(folder_id),
                    folder_path: Some(folder.path.clone()),
                    phase: "indexing".into(),
                    active: true,
                    ..Default::default()
                }
            });
            self.index_files(&folder, kind, &root, to_process, &mut stats).await;
        }
        Ok(stats)
    }

    fn retry_later(&self, folder_id: i64, paths: Vec<PathBuf>, attempt: u32) {
        if attempt >= 60 {
            log::warn!("Giving up on {} files that never settled", paths.len());
            return;
        }
        let tx = self.tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let _ = tx.send(ScanJob::Changes {
                folder_id,
                upserts: paths,
                removals: vec![],
                renames: vec![],
                attempt: attempt + 1,
            });
        });
    }

    async fn index_files(
        &mut self,
        folder: &LibraryFolder,
        kind: FolderKind,
        root: &Path,
        files: Vec<DiscoveredFile>,
        stats: &mut ScanStats,
    ) {
        let total = files.len() as u64;
        self.set_progress(|p| {
            p.phase = "indexing".into();
            p.total = total;
            p.processed = 0;
        });
        if files.is_empty() {
            return;
        }
        let ffprobe = self.ffprobe();
        let root = Arc::new(root.to_path_buf());
        let folder_id = folder.id;
        let mut stream = futures::stream::iter(files)
            .map(|f| {
                let ffprobe = ffprobe.clone();
                let root = root.clone();
                async move { extract(folder_id, kind, &root, f, ffprobe.as_deref()).await }
            })
            .buffer_unordered(4);

        let mut processed = 0u64;
        while let Some(extracted) = stream.next().await {
            processed += 1;
            let path = extracted.file.path.clone();
            if extracted.file.scan_error.is_some() {
                stats.errors += 1;
            }
            match self.write(extracted).await {
                Ok(true) => stats.added += 1,
                Ok(false) => stats.updated += 1,
                Err(e) => {
                    stats.errors += 1;
                    log::error!("Could not save {path}: {e}");
                }
            }
            let s = *stats;
            self.tick_progress(|p| {
                p.processed = processed;
                p.current_path = Some(path);
                p.added = s.added;
                p.updated = s.updated;
                p.removed = s.removed;
                p.errors = s.errors;
            });
        }
    }

    /// Persists one extracted file. Returns true when it is new to the library.
    async fn write(&self, ex: Extracted) -> Result<bool> {
        let mut is_new = media_db::find_by_path(&self.db, &ex.file.path).await?.is_none();
        if is_new {
            // A file with identical content whose old location no longer exists was
            // moved or renamed while we were not watching: keep its history.
            if let Some(fp) = ex.file.fingerprint.as_deref() {
                for (id, old_path) in media_db::find_by_fingerprint(&self.db, fp, ex.file.size).await? {
                    if !Path::new(&old_path).exists() {
                        log::info!("Relinking moved file {old_path} -> {}", ex.file.path);
                        media_db::relink(&self.db, id, &ex.file.path, ex.file.folder_id).await?;
                        is_new = false;
                        break;
                    }
                }
            }
        }
        let mut tx = self.db.pool().begin().await?;
        let (media_id, _) = media_db::upsert_media(&mut tx, &ex.file).await?;
        media_db::clear_other_kinds(&mut tx, media_id, ex.file.kind).await?;
        match &ex.data {
            KindData::Track(t) => {
                media_db::save_track(&mut tx, media_id, t).await?;
            }
            KindData::Movie(m) => {
                media_db::save_movie(&mut tx, media_id, m).await?;
            }
            KindData::Episode(e) => {
                media_db::save_episode(&mut tx, media_id, e).await?;
            }
        }
        tx.commit().await?;
        Ok(is_new)
    }
}

fn dir_name(path: Option<&Path>) -> Option<String> {
    path.and_then(|p| p.file_name()).and_then(|n| n.to_str()).map(str::to_string)
}

/// Gathers everything known about a file. Never fails: problems are recorded in
/// `scan_error` and filename-derived metadata is used instead.
async fn extract(folder_id: i64, kind: FolderKind, root: &Path, f: DiscoveredFile, ffprobe: Option<&Path>) -> Extracted {
    let path = f.path.clone();
    let fp_path = path.clone();
    let fingerprint = tokio::task::spawn_blocking(move || filesystem::fingerprint(&fp_path).ok()).await.ok().flatten();
    let media_kind = kind.media_kind();
    let mut file = FileRecord {
        folder_id,
        path: path.to_string_lossy().into_owned(),
        kind: media_kind,
        size: f.size,
        mtime: f.mtime,
        fingerprint,
        tech: TechInfo { container: crate::util::extension_lower(&path), ..Default::default() },
        scan_error: None,
    };

    let data = match media_kind {
        MediaKind::Track => {
            let p = path.clone();
            let meta = tokio::task::spawn_blocking(move || audio::read_audio_meta(&p))
                .await
                .unwrap_or_else(|e| Err(e.to_string()));
            let meta = match meta {
                Ok(m) => m,
                Err(e) => {
                    file.scan_error = Some(e);
                    Default::default()
                }
            };
            // Fall back to ffprobe for durations of exotic formats.
            if meta.duration.is_none() {
                if let Some(ffprobe) = ffprobe {
                    if let Ok(probe) = video::probe_video(ffprobe, &path).await {
                        file.tech.duration = probe.tech.duration;
                        file.tech.audio_codec = probe.tech.audio_codec;
                    }
                }
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
            let from_name = parse::parse_track_name(stem);
            let parent = path.parent().filter(|p| *p != root);
            let grandparent = parent.and_then(|p| p.parent()).filter(|p| *p != root && p.starts_with(root));
            file.tech.duration = meta.duration.or(file.tech.duration);
            file.tech.bitrate = meta.bitrate;
            if meta.codec.is_some() {
                file.tech.audio_codec = meta.codec.clone();
            }
            KindData::Track(TrackRecord {
                title: meta.title.unwrap_or(from_name.title),
                artist: meta
                    .artist
                    .clone()
                    .or(from_name.artist)
                    .or_else(|| meta.album_artist.clone())
                    .or_else(|| dir_name(grandparent))
                    .unwrap_or_else(|| "Unknown Artist".into()),
                album_artist: meta.album_artist,
                album: meta.album.or_else(|| dir_name(parent)).unwrap_or_else(|| "Unknown Album".into()),
                genre: meta.genre,
                year: meta.year,
                track_number: meta.track_number.or(from_name.track_number),
                disc_number: meta.disc_number,
            })
        }
        MediaKind::Movie | MediaKind::Episode => {
            let mut probe = video::VideoProbe::default();
            if let Some(ffprobe) = ffprobe {
                match video::probe_video(ffprobe, &path).await {
                    Ok(p) => probe = p,
                    Err(e) => file.scan_error = Some(e),
                }
            }
            file.tech = TechInfo { container: file.tech.container.clone().or(probe.tech.container.clone()), ..probe.tech.clone() };
            if media_kind == MediaKind::Movie {
                let (title, year) = parse::parse_movie_path(&path, root);
                let nfo_path = path.clone();
                let info = tokio::task::spawn_blocking(move || nfo::find_movie_nfo(&nfo_path)).await.ok().flatten();
                let info = info.unwrap_or_default();
                KindData::Movie(MovieRecord {
                    title: info.title.unwrap_or(title),
                    year: info.year.or(year).or(probe.year_tag),
                    genres: if info.genres.is_empty() {
                        probe.genre_tag.map(|g| g.split([',', '/', ';']).map(str::trim).collect::<Vec<_>>().join(","))
                    } else {
                        Some(info.genres.join(","))
                    },
                    overview: info.plot.or(probe.description_tag),
                })
            } else {
                let ep = parse::parse_episode_path(&path, root);
                let nfo_path = path.clone();
                let info = tokio::task::spawn_blocking(move || nfo::find_episode_nfo(&nfo_path)).await.ok().flatten();
                let info = info.unwrap_or_default();
                KindData::Episode(EpisodeRecord {
                    show_title: ep.show,
                    show_year: ep.show_year,
                    show_folder: ep.show_folder,
                    season_number: ep.season,
                    season_folder: ep.season_folder,
                    episode_number: ep.episode,
                    title: info.title.unwrap_or(ep.title),
                    overview: info.plot,
                })
            }
        }
    };
    Extracted { file, data }
}
