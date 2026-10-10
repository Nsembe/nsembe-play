//! Debounced recursive watching of library folders.
//!
//! Raw `notify` events are debounced (and rename pairs stitched together) by
//! `notify-debouncer-full`, then classified into upserts, removals and renames
//! that the scanner applies incrementally - no full rescans.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use notify_debouncer_full::notify::event::{ModifyKind, RenameMode};
use notify_debouncer_full::notify::{EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, DebouncedEvent, Debouncer, RecommendedCache};
use tokio::sync::mpsc::UnboundedSender;

use crate::errors::{CoreError, Result};

/// A classified filesystem change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsChange {
    /// File or directory created or modified.
    Upsert(PathBuf),
    /// File or directory removed (or moved out of view).
    Remove(PathBuf),
    /// File or directory renamed/moved.
    Rename(PathBuf, PathBuf),
}

pub struct FolderWatcher {
    debouncer: Mutex<Debouncer<RecommendedWatcher, RecommendedCache>>,
    watched: Mutex<HashSet<PathBuf>>,
}

impl FolderWatcher {
    pub fn new(tx: UnboundedSender<Vec<FsChange>>) -> Result<Self> {
        let debouncer = new_debouncer(Duration::from_millis(1500), None, move |res: DebounceEventResult| match res {
            Ok(events) => {
                let changes = classify(&events);
                if !changes.is_empty() {
                    let _ = tx.send(changes);
                }
            }
            Err(errors) => {
                for e in errors {
                    log::warn!("Folder watcher error: {e}");
                }
            }
        })
        .map_err(|e| CoreError::Other(format!("Could not start folder watcher: {e}")))?;
        Ok(FolderWatcher { debouncer: Mutex::new(debouncer), watched: Mutex::new(HashSet::new()) })
    }

    pub fn watch(&self, path: &Path) -> Result<()> {
        let mut watched = self.watched.lock().unwrap();
        if watched.contains(path) {
            return Ok(());
        }
        self.debouncer
            .lock()
            .unwrap()
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| CoreError::Other(format!("Could not watch {}: {e}", path.display())))?;
        watched.insert(path.to_path_buf());
        log::info!("Watching {}", path.display());
        Ok(())
    }

    pub fn unwatch(&self, path: &Path) {
        if self.watched.lock().unwrap().remove(path) {
            if let Err(e) = self.debouncer.lock().unwrap().unwatch(path) {
                log::warn!("Could not unwatch {}: {e}", path.display());
            }
        }
    }

    pub fn unwatch_all(&self) {
        let paths: Vec<PathBuf> = self.watched.lock().unwrap().iter().cloned().collect();
        for p in paths {
            self.unwatch(&p);
        }
    }
}

/// Converts debounced notify events into simple change records.
pub fn classify(events: &[DebouncedEvent]) -> Vec<FsChange> {
    let mut out = Vec::new();
    for ev in events {
        let paths = &ev.event.paths;
        match ev.event.kind {
            EventKind::Create(_) => out.extend(paths.iter().cloned().map(FsChange::Upsert)),
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) if paths.len() >= 2 => {
                out.push(FsChange::Rename(paths[0].clone(), paths[1].clone()));
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                out.extend(paths.iter().cloned().map(FsChange::Remove))
            }
            EventKind::Modify(ModifyKind::Name(_)) => {
                // "To" or unknown rename: re-evaluate whatever exists now.
                for p in paths {
                    if p.exists() {
                        out.push(FsChange::Upsert(p.clone()));
                    } else {
                        out.push(FsChange::Remove(p.clone()));
                    }
                }
            }
            EventKind::Modify(ModifyKind::Metadata(_)) => {}
            EventKind::Modify(_) => out.extend(paths.iter().cloned().map(FsChange::Upsert)),
            EventKind::Remove(_) => out.extend(paths.iter().cloned().map(FsChange::Remove)),
            EventKind::Any | EventKind::Other | EventKind::Access(_) => {}
        }
    }
    out.dedup();
    out
}
