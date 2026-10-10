//! End-to-end tests of the library engine against a temporary media tree.
//! External tools (ffprobe/ffmpeg/mpv) are deliberately disabled so the tests are
//! deterministic and exercise the filename/.nfo fallbacks.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use nsembe_core::database::{favorites, home, media as media_db, music, playback, playlists, search, video, Db};
use nsembe_core::events::{CoreEvent, EventSink};
use nsembe_core::library::Library;
use nsembe_core::models::FolderKind;
use nsembe_core::scanner::Scanner;
use nsembe_core::thumbnails::{ArtworkGenerator, ArtworkStore};
use nsembe_core::tools::ToolPaths;
use tokio::sync::mpsc;

struct ChannelSink(mpsc::UnboundedSender<CoreEvent>);

impl EventSink for ChannelSink {
    fn emit(&self, event: CoreEvent) {
        let _ = self.0.send(event);
    }
}

struct Fixture {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    db: Db,
    library: Arc<Library>,
    events: mpsc::UnboundedReceiver<CoreEvent>,
}

fn write_file(path: &Path, seed: u8) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    // Distinct content per file so fingerprints differ.
    let mut data = vec![seed; 4096];
    data.extend(path.to_string_lossy().as_bytes());
    std::fs::write(path, data).unwrap();
}

async fn setup() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(tmp.path()).unwrap();
    let m = root.join("Music");
    write_file(&m.join("Pink Floyd/The Dark Side of the Moon/01 - Speak to Me.mp3"), 1);
    write_file(&m.join("Pink Floyd/The Dark Side of the Moon/02 - Breathe.mp3"), 2);
    write_file(&m.join("Radiohead/In Rainbows/07 - Reckoner.flac"), 3);
    std::fs::write(m.join("Radiohead/In Rainbows/notes.txt"), "not media").unwrap();

    let mv = root.join("Movies");
    write_file(&mv.join("Inception (2010).mkv"), 10);
    write_file(&mv.join("Arrival (2016)/arrival.1080p.mkv"), 11);
    std::fs::write(
        mv.join("Arrival (2016)/movie.nfo"),
        "<movie><title>Arrival</title><year>2016</year><plot>Linguist meets aliens.</plot><genre>Science Fiction</genre><genre>Drama</genre></movie>",
    )
    .unwrap();
    write_file(&mv.join("Interstellar.2014.1080p.BluRay.x264.mp4"), 12);
    std::fs::write(
        mv.join("Interstellar.2014.1080p.BluRay.x264.nfo"),
        "<movie><title>Interstellar</title><genre>Science Fiction</genre><genre>Adventure</genre></movie>",
    )
    .unwrap();
    write_file(&mv.join("sample.mkv"), 13);

    let tv = root.join("TV");
    for (s, e) in [(1, 1), (1, 2), (1, 3), (2, 1)] {
        write_file(&tv.join(format!("Show Name/Season {s:02}/S{s:02}E{e:02}.mkv")), 20 + e as u8 + s as u8 * 5);
    }

    let db = Db::open(&root.join("data/library.db")).await.unwrap();
    let (tx, events) = mpsc::unbounded_channel();
    let sink: Arc<dyn EventSink> = Arc::new(ChannelSink(tx));
    let tools = Arc::new(RwLock::new(ToolPaths::default()));
    let store = ArtworkStore::new(root.join("data/artwork")).unwrap();
    let artwork = Arc::new(ArtworkGenerator::new(db.clone(), store, tools.clone(), sink.clone()));
    let scanner = Scanner::start(db.clone(), sink.clone(), tools, artwork);
    let library = Library::start(db.clone(), sink, scanner).await.unwrap();
    Fixture { _tmp: tmp, root, db, library, events }
}

impl Fixture {
    /// Waits until the scanner reports it is idle again.
    async fn wait_idle(&mut self) {
        let res = tokio::time::timeout(Duration::from_secs(30), async {
            while let Some(ev) = self.events.recv().await {
                if let CoreEvent::ScanProgress(p) = ev {
                    if p.phase == "idle" {
                        return;
                    }
                }
            }
        })
        .await;
        assert!(res.is_ok(), "scanner did not become idle");
    }

    async fn add_all(&mut self) {
        for (dir, kind) in [("Music", FolderKind::Music), ("Movies", FolderKind::Movies), ("TV", FolderKind::Tv)] {
            self.library.add_folder(&self.root.join(dir).to_string_lossy(), kind).await.unwrap();
            self.wait_idle().await;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scans_music_movies_and_tv() {
    let mut fx = setup().await;
    fx.add_all().await;

    let stats = home::stats(&fx.db).await.unwrap();
    assert_eq!(stats.tracks, 3);
    assert_eq!(stats.albums, 2);
    assert_eq!(stats.movies, 3, "sample.mkv must be ignored");
    assert_eq!(stats.shows, 1);
    assert_eq!(stats.episodes, 4);

    // Untagged music falls back to the Artist/Album/NN - Title layout.
    let albums = music::list_albums(&fx.db, "title", 0, 50).await.unwrap();
    let dsotm = albums.items.iter().find(|a| a.title == "The Dark Side of the Moon").expect("album");
    assert_eq!(dsotm.artist_name, "Pink Floyd");
    let tracks = music::album_tracks(&fx.db, dsotm.id).await.unwrap();
    assert_eq!(tracks.iter().map(|t| t.title.as_str()).collect::<Vec<_>>(), vec!["Speak to Me", "Breathe"]);
    assert_eq!(tracks[0].track_number, Some(1));

    // Movies: filename, folder name and .nfo metadata.
    let movies = video::list_movies(&fx.db, "title", None, 0, 50).await.unwrap();
    let titles: Vec<_> = movies.items.iter().map(|m| (m.title.as_str(), m.year)).collect();
    assert!(titles.contains(&("Inception", Some(2010))));
    assert!(titles.contains(&("Arrival", Some(2016))));
    assert!(titles.contains(&("Interstellar", Some(2014))));
    let arrival = movies.items.iter().find(|m| m.title == "Arrival").unwrap();
    assert_eq!(arrival.genres.as_deref(), Some("Science Fiction,Drama"));

    // Genre-based "more like this".
    let detail = video::movie_detail(&fx.db, arrival.id).await.unwrap();
    assert_eq!(detail.more_like_this.first().map(|m| m.title.as_str()), Some("Interstellar"));
    let scifi = video::list_movies(&fx.db, "title", Some("Science Fiction"), 0, 50).await.unwrap();
    assert_eq!(scifi.total, 2);

    // TV structure.
    let shows = video::list_shows(&fx.db, "title", 0, 10).await.unwrap();
    let show = &shows.items[0];
    assert_eq!(show.title, "Show Name");
    let detail = video::show_detail(&fx.db, show.id).await.unwrap();
    assert_eq!(detail.seasons.len(), 2);
    assert_eq!(detail.episodes.len(), 4);
    assert_eq!(detail.next_up.as_ref().map(|e| (e.season_number, e.episode_number)), Some((1, 1)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn incremental_rescan_missing_and_moved_files() {
    let mut fx = setup().await;
    fx.add_all().await;
    let movies = video::list_movies(&fx.db, "title", None, 0, 50).await.unwrap();
    let inception = movies.items.iter().find(|m| m.title == "Inception").unwrap().clone();
    playback::save_progress(&fx.db, inception.media_id, 600.0, Some(8880.0), false).await.unwrap();

    // Unchanged rescan does not touch anything.
    let before = media_db::existing_for_folder(&fx.db, 2).await.unwrap();
    fx.library.rescan(None, false).await.unwrap();
    fx.wait_idle().await;
    let after = media_db::existing_for_folder(&fx.db, 2).await.unwrap();
    assert_eq!(before.len(), after.len());

    // Delete a file: it becomes missing and disappears from views, but is not purged.
    let old = fx.root.join("Movies/Inception (2010).mkv");
    let content = std::fs::read(&old).unwrap();
    std::fs::remove_file(&old).unwrap();
    fx.library.rescan(None, false).await.unwrap();
    fx.wait_idle().await;
    let movies = video::list_movies(&fx.db, "title", None, 0, 50).await.unwrap();
    assert!(!movies.items.iter().any(|m| m.title == "Inception"));

    // Same content appears elsewhere: relinked to the same media row, history kept.
    let new = fx.root.join("Movies/Inception (2010)/Inception (2010).mkv");
    std::fs::create_dir_all(new.parent().unwrap()).unwrap();
    std::fs::write(&new, content).unwrap();
    fx.library.rescan(None, false).await.unwrap();
    fx.wait_idle().await;
    let movies = video::list_movies(&fx.db, "title", None, 0, 50).await.unwrap();
    let moved = movies.items.iter().find(|m| m.title == "Inception").expect("relinked");
    assert_eq!(moved.media_id, inception.media_id);
    assert_eq!(moved.position, Some(600.0));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn continue_watching_and_next_episode() {
    let mut fx = setup().await;
    fx.add_all().await;
    let movie = video::list_movies(&fx.db, "title", None, 0, 50).await.unwrap().items.remove(0);
    let show = video::list_shows(&fx.db, "title", 0, 10).await.unwrap().items.remove(0);
    let episodes = video::show_detail(&fx.db, show.id).await.unwrap().episodes;

    // Half-watched movie -> continue watching.
    playback::record_play_start(&fx.db, movie.media_id, Some(6000.0)).await.unwrap();
    let completed = playback::save_progress(&fx.db, movie.media_id, 3000.0, Some(6000.0), false).await.unwrap();
    assert!(!completed);
    let cw = playback::continue_watching(&fx.db, 20).await.unwrap();
    assert!(cw.iter().any(|c| c.item.media_id == movie.media_id && c.reason == "resume"));
    assert_eq!(playback::resume_position(&fx.db, movie.media_id).await.unwrap(), Some(3000.0));

    // Watching past 90% completes it and removes it from continue watching.
    assert!(playback::save_progress(&fx.db, movie.media_id, 5500.0, Some(6000.0), false).await.unwrap());
    let cw = playback::continue_watching(&fx.db, 20).await.unwrap();
    assert!(!cw.iter().any(|c| c.item.media_id == movie.media_id));
    assert_eq!(playback::resume_position(&fx.db, movie.media_id).await.unwrap(), None);

    // Finishing S01E01 offers S01E02 next.
    let e1 = &episodes[0];
    playback::record_play_start(&fx.db, e1.media_id, Some(1500.0)).await.unwrap();
    playback::save_progress(&fx.db, e1.media_id, 1500.0, Some(1500.0), true).await.unwrap();
    let cw = playback::continue_watching(&fx.db, 20).await.unwrap();
    let next = cw.iter().find(|c| c.item.show_id == Some(show.id)).expect("next episode entry");
    assert_eq!(next.reason, "next");
    assert_eq!(next.item.media_id, episodes[1].media_id);
    let up = video::show_next_up(&fx.db, show.id).await.unwrap().unwrap();
    assert_eq!((up.season_number, up.episode_number), (1, 2));

    // Last episode of season 1 continues into season 2.
    let s1e3 = video::episode(&fx.db, episodes[2].id).await.unwrap();
    let next = video::next_episode(&fx.db, &s1e3).await.unwrap().unwrap();
    assert_eq!((next.season_number, next.episode_number), (2, 1));
    let prev = video::previous_episode(&fx.db, &next).await.unwrap().unwrap();
    assert_eq!(prev.id, s1e3.id);

    let recent = playback::recently_played(&fx.db, None, 0, 10).await.unwrap();
    assert_eq!(recent.total, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn playlists_and_favorites() {
    let mut fx = setup().await;
    fx.add_all().await;
    let tracks = music::list_tracks(&fx.db, "title", 0, 50).await.unwrap().items;
    let movie = video::list_movies(&fx.db, "title", None, 0, 50).await.unwrap().items.remove(0);

    let p = playlists::create(&fx.db, "  Road Trip ", Some("Songs for the drive")).await.unwrap();
    assert_eq!(p.name, "Road Trip");
    assert!(playlists::create(&fx.db, "   ", None).await.is_err());

    let ids: Vec<i64> = tracks.iter().map(|t| t.media_id).chain([movie.media_id]).collect();
    assert_eq!(playlists::add_items(&fx.db, p.id, &ids).await.unwrap(), 4);
    assert_eq!(playlists::add_items(&fx.db, p.id, &ids[..1]).await.unwrap(), 0, "duplicates skipped");

    let detail = playlists::detail(&fx.db, p.id).await.unwrap();
    assert_eq!(detail.entries.len(), 4);
    assert_eq!(detail.playlist.item_count, 4);

    // Reverse the order.
    let reversed: Vec<i64> = detail.entries.iter().rev().map(|e| e.item_id).collect();
    playlists::reorder(&fx.db, p.id, &reversed).await.unwrap();
    let detail = playlists::detail(&fx.db, p.id).await.unwrap();
    assert_eq!(detail.entries.iter().map(|e| e.item_id).collect::<Vec<_>>(), reversed);
    assert_eq!(detail.entries[0].media.kind, "movie");

    playlists::remove_item(&fx.db, reversed[0]).await.unwrap();
    assert_eq!(playlists::media_ids(&fx.db, p.id).await.unwrap().len(), 3);

    let renamed = playlists::update(&fx.db, p.id, "Weekend", None).await.unwrap();
    assert_eq!(renamed.name, "Weekend");
    assert_eq!(renamed.description, None);

    // Favorites
    assert!(favorites::toggle(&fx.db, "track", tracks[0].id).await.unwrap());
    assert!(favorites::toggle(&fx.db, "movie", movie.id).await.unwrap());
    assert!(favorites::toggle(&fx.db, "bogus", 1).await.is_err());
    let fav = favorites::overview(&fx.db).await.unwrap();
    assert_eq!(fav.tracks.len(), 1);
    assert!(fav.tracks[0].is_favorite);
    assert_eq!(fav.movies.len(), 1);
    assert!(!favorites::toggle(&fx.db, "movie", movie.id).await.unwrap());

    // Removing the music folder removes its tracks, their favorites and playlist entries.
    let folders = fx.library.folders().await.unwrap();
    let music_folder = folders.iter().find(|f| f.kind == "music").unwrap();
    fx.library.remove_folder(music_folder.id).await.unwrap();
    assert!(favorites::overview(&fx.db).await.unwrap().tracks.is_empty());
    assert_eq!(playlists::media_ids(&fx.db, p.id).await.unwrap().len(), 0);
    assert_eq!(home::stats(&fx.db).await.unwrap().artists, 0);

    playlists::delete(&fx.db, p.id).await.unwrap();
    assert!(playlists::list(&fx.db).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fuzzy_search_groups_results() {
    let mut fx = setup().await;
    fx.add_all().await;
    playlists::create(&fx.db, "Interstellar Soundtrack", None).await.unwrap();

    let r = search::search(&fx.db, "intersteller", 8).await.unwrap();
    assert_eq!(r.movies.first().map(|m| m.title.as_str()), Some("Interstellar"));
    assert_eq!(r.playlists.len(), 1);

    let r = search::search(&fx.db, "pink floid", 8).await.unwrap();
    assert_eq!(r.artists.first().map(|a| a.name.as_str()), Some("Pink Floyd"));

    let r = search::search(&fx.db, "breath", 8).await.unwrap();
    assert_eq!(r.tracks.first().map(|t| t.title.as_str()), Some("Breathe"));

    let r = search::search(&fx.db, "show nme", 8).await.unwrap();
    assert_eq!(r.shows.len(), 1);

    let r = search::search(&fx.db, "zzzzqqq", 8).await.unwrap();
    assert!(r.tracks.is_empty() && r.movies.is_empty() && r.artists.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rejects_duplicate_and_nested_folders() {
    let mut fx = setup().await;
    fx.add_all().await;
    let music = fx.root.join("Music");
    assert!(fx.library.add_folder(&music.to_string_lossy(), FolderKind::Music).await.is_err());
    assert!(fx.library.add_folder(&music.join("Pink Floyd").to_string_lossy(), FolderKind::Music).await.is_err());
    assert!(fx.library.add_folder(&fx.root.to_string_lossy(), FolderKind::Music).await.is_err());
    assert!(fx.library.add_folder(&fx.root.join("nope").to_string_lossy(), FolderKind::Music).await.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn watcher_picks_up_new_and_deleted_files() {
    let mut fx = setup().await;
    fx.add_all().await;

    write_file(&fx.root.join("Movies/Heat (1995).mkv"), 99);
    let found = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let movies = video::list_movies(&fx.db, "title", None, 0, 50).await.unwrap();
            if movies.items.iter().any(|m| m.title == "Heat") {
                return;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    })
    .await;
    assert!(found.is_ok(), "new file was not detected by the watcher");

    std::fs::remove_file(fx.root.join("Movies/Heat (1995).mkv")).unwrap();
    let gone = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let movies = video::list_movies(&fx.db, "title", None, 0, 50).await.unwrap();
            if !movies.items.iter().any(|m| m.title == "Heat") {
                return;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    })
    .await;
    assert!(gone.is_ok(), "deleted file was not detected by the watcher");
    // Drain events so the channel does not grow unbounded in long runs.
    while fx.events.try_recv().is_ok() {}
}
