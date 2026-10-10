//! Library management: folders, scanning triggers and filesystem watching.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::database::{folders, media as media_db, Db};
use crate::errors::{CoreError, Result};
use crate::events::{CoreEvent, EventSink};
use crate::filesystem::watcher::{FolderWatcher, FsChange};
use crate::models::{FolderKind, LibraryFolder, ScanProgress};
use crate::scanner::{ScanJob, Scanner};
use crate::settings::{self, Settings};
use crate::util::now;

/// Missing items are forgotten after this long (they may come back, e.g. a USB drive).
const MISSING_RETENTION_SECS: i64 = 30 * 24 * 3600;

pub struct Library {
    db: Db,
    sink: Arc<dyn EventSink>,
    scanner: Scanner,
    settings: Settings,
    watcher: Option<FolderWatcher>,
}

impl Library {
    pub async fn start(db: Db, sink: Arc<dyn EventSink>, scanner: Scanner) -> Result<Arc<Self>> {
        let settings = Settings::new(db.clone());
        let (tx, rx) = mpsc::unbounded_channel();
        let watcher = match FolderWatcher::new(tx) {
            Ok(w) => Some(w),
            Err(e) => {
                log::error!("{e}");
                None
            }
        };
        let library = Arc::new(Library { db, sink, scanner, settings, watcher });
        tokio::spawn(Self::route_changes(library.db.clone(), library.scanner.clone(), rx));

        if let Err(e) = media_db::purge_missing(&library.db, Some(now() - MISSING_RETENTION_SECS)).await {
            log::warn!("Purging long-missing media failed: {e}");
        }
        // Catch up with changes made while the app was closed (incremental, cheap).
        let watch = library.settings.get_bool(settings::WATCH_FOLDERS).await;
        for folder in folders::list(&library.db).await? {
            if watch {
                library.watch(&folder);
            }
            library.scanner.enqueue(ScanJob::Full { folder_id: folder.id, force: false });
        }
        Ok(library)
    }

    fn watch(&self, folder: &LibraryFolder) {
        if let Some(w) = &self.watcher {
            if Path::new(&folder.path).is_dir() {
                if let Err(e) = w.watch(Path::new(&folder.path)) {
                    log::warn!("{e}");
                }
            }
        }
    }

    /// Maps watcher changes to the library folder they belong to.
    async fn route_changes(db: Db, scanner: Scanner, mut rx: mpsc::UnboundedReceiver<Vec<FsChange>>) {
        while let Some(changes) = rx.recv().await {
            let folders = match folders::list(&db).await {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Could not load folders for watcher event: {e}");
                    continue;
                }
            };
            let roots: Vec<(i64, PathBuf)> = folders.iter().map(|f| (f.id, PathBuf::from(&f.path))).collect();
            let owner = |p: &Path| -> Option<i64> {
                roots
                    .iter()
                    .filter(|(_, root)| p.starts_with(root))
                    .max_by_key(|(_, root)| root.as_os_str().len())
                    .map(|(id, _)| *id)
            };
            type Batch = (Vec<PathBuf>, Vec<PathBuf>, Vec<(PathBuf, PathBuf)>);
            let mut per_folder: HashMap<i64, Batch> = HashMap::new();
            for change in changes {
                match change {
                    FsChange::Upsert(p) => {
                        if let Some(id) = owner(&p) {
                            per_folder.entry(id).or_default().0.push(p);
                        }
                    }
                    FsChange::Remove(p) => {
                        if let Some(id) = owner(&p) {
                            per_folder.entry(id).or_default().1.push(p);
                        }
                    }
                    FsChange::Rename(from, to) => match (owner(&from), owner(&to)) {
                        (Some(a), Some(b)) if a == b => per_folder.entry(a).or_default().2.push((from, to)),
                        (a, b) => {
                            // Moved between folders (or in/out of the library): the new
                            // location is matched back by content fingerprint.
                            if let Some(a) = a {
                                per_folder.entry(a).or_default().1.push(from);
                            }
                            if let Some(b) = b {
                                per_folder.entry(b).or_default().0.push(to);
                            }
                        }
                    },
                }
            }
            for (folder_id, (upserts, removals, renames)) in per_folder {
                scanner.enqueue(ScanJob::Changes { folder_id, upserts, removals, renames, attempt: 0 });
            }
        }
    }

    pub async fn folders(&self) -> Result<Vec<LibraryFolder>> {
        folders::list(&self.db).await
    }

    /// Adds a folder to the library and starts indexing it in the background.
    pub async fn add_folder(&self, path: &str, kind: FolderKind) -> Result<LibraryFolder> {
        let raw = PathBuf::from(path.trim());
        if raw.as_os_str().is_empty() {
            return Err(CoreError::invalid("Choose a folder first"));
        }
        if !raw.is_dir() {
            return Err(CoreError::invalid(format!("{} is not an accessible folder", raw.display())));
        }
        std::fs::read_dir(&raw).map_err(|e| CoreError::invalid(format!("Nsembe Play cannot read {}: {e}", raw.display())))?;
        let canonical = dunce::canonicalize(&raw).unwrap_or(raw);
        for existing in folders::list(&self.db).await? {
            let other = PathBuf::from(&existing.path);
            if other == canonical {
                return Err(CoreError::invalid("This folder is already in your library"));
            }
            if canonical.starts_with(&other) {
                return Err(CoreError::invalid(format!("This folder is already included through {}", other.display())));
            }
            if other.starts_with(&canonical) {
                return Err(CoreError::invalid(format!(
                    "{} is inside this folder. Remove it first, then add the parent folder.",
                    other.display()
                )));
            }
        }
        let id = folders::insert(&self.db, &canonical.to_string_lossy(), kind).await?;
        let folder = folders::get(&self.db, id).await?;
        if self.settings.get_bool(settings::WATCH_FOLDERS).await {
            self.watch(&folder);
        }
        self.scanner.enqueue(ScanJob::Full { folder_id: id, force: false });
        Ok(folder)
    }

    pub async fn remove_folder(&self, id: i64) -> Result<()> {
        let folder = folders::get(&self.db, id).await?;
        if let Some(w) = &self.watcher {
            w.unwatch(Path::new(&folder.path));
        }
        folders::delete(&self.db, id).await?;
        media_db::prune_orphans(&self.db).await?;
        self.sink.emit(CoreEvent::LibraryChanged { reason: "folder_removed".into() });
        Ok(())
    }

    /// Re-scans one folder or all of them. `force` re-reads metadata for every file.
    pub async fn rescan(&self, folder_id: Option<i64>, force: bool) -> Result<()> {
        match folder_id {
            Some(id) => self.scanner.enqueue(ScanJob::Full { folder_id: id, force }),
            None => {
                for f in folders::list(&self.db).await? {
                    self.scanner.enqueue(ScanJob::Full { folder_id: f.id, force });
                }
            }
        }
        Ok(())
    }

    pub fn regenerate_artwork(&self) {
        self.scanner.enqueue(ScanJob::Artwork);
    }

    /// Forgets all files currently marked missing.
    pub async fn cleanup_missing(&self) -> Result<u64> {
        let n = media_db::purge_missing(&self.db, None).await?;
        self.sink.emit(CoreEvent::LibraryChanged { reason: "cleanup".into() });
        Ok(n)
    }

    pub async fn set_watching(&self, enabled: bool) -> Result<()> {
        let Some(w) = &self.watcher else { return Ok(()) };
        if enabled {
            for f in folders::list(&self.db).await? {
                self.watch(&f);
            }
        } else {
            w.unwatch_all();
        }
        Ok(())
    }

    pub fn scan_progress(&self) -> ScanProgress {
        self.scanner.progress()
    }
}
