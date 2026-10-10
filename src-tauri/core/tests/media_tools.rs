//! Integration tests that exercise the real external tools: FFmpeg generates
//! media, FFprobe/FFmpeg index it, and mpv plays it over its IPC protocol.
//!
//! The tools are looked up in `src-tauri/resources/bin` (see
//! scripts/fetch-tools.ps1) or `NSEMBE_TOOLS_DIR`. Without them the tests are
//! skipped, so the suite stays green on machines without the tools.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use nsembe_core::database::{media as media_db, music, playback, video, Db};
use nsembe_core::events::{CoreEvent, EventSink};
use nsembe_core::library::Library;
use nsembe_core::models::{EngineKind, FolderKind};
use nsembe_core::player::{PlayOptions, PlayerCommand, PlayerManager};
use nsembe_core::scanner::Scanner;
use nsembe_core::settings::{self, Settings};
use nsembe_core::thumbnails::{ArtworkGenerator, ArtworkStore};
use nsembe_core::tools::{self, ToolOverrides, ToolPaths};
use serde_json::json;

fn tool_dir() -> PathBuf {
    std::env::var("NSEMBE_TOOLS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("resources").join("bin"))
}

fn find_tools() -> Option<ToolPaths> {
    let found = tools::locate_all(&ToolOverrides::default(), &[tool_dir()]);
    if found.mpv.is_some() && found.ffmpeg.is_some() && found.ffprobe.is_some() {
        Some(found)
    } else {
        eprintln!("skipping: mpv/ffmpeg/ffprobe not found (run scripts/fetch-tools.ps1)");
        None
    }
}

fn ffmpeg(ffmpeg: &Path, args: &[&str]) {
    let status = Command::new(ffmpeg).args(["-hide_banner", "-loglevel", "error", "-y"]).args(args).status().unwrap();
    assert!(status.success(), "ffmpeg failed: {args:?}");
}

#[derive(Default)]
struct Recorder(Mutex<Vec<CoreEvent>>);

impl EventSink for Recorder {
    fn emit(&self, event: CoreEvent) {
        self.0.lock().unwrap().push(event);
    }
}

impl Recorder {
    fn idle_count(&self) -> usize {
        self.0.lock().unwrap().iter().filter(|e| matches!(e, CoreEvent::ScanProgress(p) if p.phase == "idle")).count()
    }
}

async fn wait_until(what: &str, secs: u64, mut f: impl FnMut() -> bool) {
    let ok = tokio::time::timeout(Duration::from_secs(secs), async {
        while !f() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;
    assert!(ok.is_ok(), "timed out waiting for {what}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scan_with_ffprobe_and_play_with_mpv() {
    let Some(found) = find_tools() else { return };
    let ff = found.ffmpeg.clone().unwrap();

    let tmp = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(tmp.path()).unwrap();
    let music_dir = root.join("Music").join("Test Artist").join("Test Album");
    let movie_dir = root.join("Movies").join("Test Pattern (2024)");
    std::fs::create_dir_all(&music_dir).unwrap();
    std::fs::create_dir_all(&movie_dir).unwrap();

    // A tagged 4 second song and a 12 second test-pattern movie.
    let song = music_dir.join("01 - Tone.mp3");
    ffmpeg(&ff, &[
        "-f", "lavfi", "-i", "sine=frequency=440:duration=4", "-metadata", "title=Concert A", "-metadata", "artist=Tone Generator",
        "-metadata", "album=Calibration", "-metadata", "track=1", "-metadata", "genre=Test", "-metadata", "date=2024",
        song.to_str().unwrap(),
    ]);
    let movie = movie_dir.join("pattern.mp4");
    ffmpeg(&ff, &[
        "-f", "lavfi", "-i", "testsrc=duration=12:size=640x360:rate=25", "-f", "lavfi", "-i", "sine=frequency=220:duration=12",
        "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest", movie.to_str().unwrap(),
    ]);

    let db = Db::open(&root.join("data/library.db")).await.unwrap();
    Settings::new(db.clone()).set(settings::VOLUME, json!(0)).await.unwrap(); // keep the test silent
    let recorder = Arc::new(Recorder::default());
    let sink: Arc<dyn EventSink> = recorder.clone();
    let tool_paths = Arc::new(RwLock::new(found));
    let artwork = Arc::new(ArtworkGenerator::new(db.clone(), ArtworkStore::new(root.join("data/artwork")).unwrap(), tool_paths.clone(), sink.clone()));
    let scanner = Scanner::start(db.clone(), sink.clone(), tool_paths.clone(), artwork);
    let library = Library::start(db.clone(), sink.clone(), scanner).await.unwrap();

    library.add_folder(&root.join("Music").to_string_lossy(), FolderKind::Music).await.unwrap();
    library.add_folder(&root.join("Movies").to_string_lossy(), FolderKind::Movies).await.unwrap();
    wait_until("scans to finish", 60, || recorder.idle_count() >= 1).await;
    // Let any queued second job (the movies folder) complete as well.
    let movies_ready = || async { video::list_movies(&db, "title", None, 0, 10).await.unwrap().total == 1 };
    for _ in 0..100 {
        if movies_ready().await {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Embedded tags were read.
    let tracks = music::list_tracks(&db, "title", 0, 10).await.unwrap().items;
    assert_eq!(tracks.len(), 1);
    let track = &tracks[0];
    assert_eq!(track.title, "Concert A");
    assert_eq!(track.artist_name.as_deref(), Some("Tone Generator"));
    assert_eq!(track.album_title.as_deref(), Some("Calibration"));
    assert!((track.duration.unwrap() - 4.0).abs() < 0.5);

    // FFprobe metadata and folder-name title.
    let movies = video::list_movies(&db, "title", None, 0, 10).await.unwrap().items;
    assert_eq!(movies.len(), 1);
    let mv = &movies[0];
    assert_eq!((mv.title.as_str(), mv.year), ("Test Pattern", Some(2024)));
    assert_eq!((mv.width, mv.height), (Some(640), Some(360)));
    assert_eq!(mv.video_codec.as_deref(), Some("h264"));
    assert!((mv.duration.unwrap() - 12.0).abs() < 1.0);

    // FFmpeg thumbnail generated in the background artwork pass.
    let mut thumb = None;
    for _ in 0..100 {
        thumb = video::movie(&db, mv.id).await.unwrap().backdrop;
        if thumb.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let thumb = thumb.expect("thumbnail was generated");
    assert!(Path::new(&thumb).is_file());

    // --- mpv playback over IPC -------------------------------------------------
    let player = PlayerManager::new(db.clone(), sink.clone(), tool_paths.clone(), root.join("data"));
    let item = player.play(track.media_id, PlayOptions::default()).await.expect("mpv starts");
    assert_eq!(item.media_id, track.media_id);

    let p = player.clone();
    wait_until("playback position to advance", 20, || p.state(EngineKind::Audio).position > 0.8).await;
    let state = player.state(EngineKind::Audio);
    assert!(state.active);
    assert!((state.duration.unwrap() - 4.0).abs() < 0.5);

    player.command(EngineKind::Audio, PlayerCommand::Pause).await.unwrap();
    let p = player.clone();
    wait_until("pause to be reflected", 10, || p.state(EngineKind::Audio).paused).await;
    // Pausing persists the position.
    let mut saved = 0.0;
    for _ in 0..50 {
        saved = playback::recently_played(&db, Some("music"), 0, 5).await.unwrap().items[0].position.unwrap_or(0.0);
        if saved > 0.5 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(saved > 0.5, "position persisted on pause (got {saved})");

    // Seek near the end and let it finish: the item is marked completed and an
    // "ended" event is emitted for the queue to advance.
    player.command(EngineKind::Audio, PlayerCommand::Seek { position: 3.0 }).await.unwrap();
    player.command(EngineKind::Audio, PlayerCommand::Resume).await.unwrap();
    let rec = recorder.clone();
    wait_until("end of file", 20, || {
        rec.0.lock().unwrap().iter().any(|e| matches!(e, CoreEvent::PlayerEnded(x) if x.reason == "eof" && x.media_id == track.media_id))
    })
    .await;
    let mut completed = false;
    for _ in 0..50 {
        completed = playback::recently_played(&db, Some("music"), 0, 5).await.unwrap().items[0].completed == Some(true);
        if completed {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(completed, "track marked completed after EOF");

    // A missing file is reported clearly and flagged in the library.
    std::fs::remove_file(&movie).unwrap();
    let err = player.play(mv.media_id, PlayOptions::default()).await.unwrap_err();
    assert_eq!(err.code(), "missing_file");
    assert!(media_db::find_by_path(&db, &movie.to_string_lossy()).await.unwrap().unwrap().missing);

    player.shutdown().await;
    let p = player.clone();
    wait_until("mpv to exit", 10, || !p.state(EngineKind::Audio).active).await;
}
