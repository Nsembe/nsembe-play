//! Playback control.
//!
//! Two independent mpv engines are managed: an audio engine (headless, used for
//! music) and a video engine (its own window). Playback positions are persisted
//! periodically and on pause/stop/exit so viewers can always resume.
//!
//! Clients that play media themselves (e.g. a built-in HTML player) report
//! progress through [`PlayerManager::report_external`] so the same history,
//! resume and "continue watching" logic applies.

pub mod mpv;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::database::{media as media_db, playback, video, Db};
use crate::errors::{CoreError, Result};
use crate::events::{CoreEvent, EventSink, Notice};
use crate::models::{EngineKind, MediaKind, PlayableItem, PlayerEnded, PlayerState, TrackInfo};
use crate::settings::{self, Settings};
use crate::tools::ToolPaths;
use mpv::{MpvEvent, MpvProcess};

const PERSIST_INTERVAL: Duration = Duration::from_secs(5);
const EMIT_INTERVAL: Duration = Duration::from_millis(250);

/// Commands accepted by [`PlayerManager::command`].
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PlayerCommand {
    Pause,
    Resume,
    TogglePause,
    Seek { position: f64 },
    SeekRelative { offset: f64 },
    SetVolume { volume: f64 },
    SetMuted { muted: bool },
    SetSpeed { speed: f64 },
    SetAudioTrack { id: i64 },
    SetSubtitleTrack { id: Option<i64> },
    SetFullscreen { fullscreen: bool },
    Stop,
}

/// Progress reports from a client-side player.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExternalEvent {
    Start,
    Progress,
    Pause,
    Stop,
    Ended,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayOptions {
    /// Start position in seconds. `None` resumes where the user left off.
    pub start: Option<f64>,
    /// mpv `--geometry` for the video window (e.g. to cover the app window).
    pub geometry: Option<String>,
}

struct Slot {
    process: Option<Arc<MpvProcess>>,
    generation: u64,
    state: PlayerState,
    loaded: bool,
    ended_handled: bool,
    last_persist: Instant,
    last_emit: Instant,
}

impl Slot {
    fn new(engine: EngineKind) -> Self {
        Slot {
            process: None,
            generation: 0,
            state: PlayerState::idle(engine),
            loaded: false,
            ended_handled: false,
            last_persist: Instant::now(),
            last_emit: Instant::now(),
        }
    }
}

pub struct PlayerManager {
    db: Db,
    settings: Settings,
    sink: Arc<dyn EventSink>,
    tools: Arc<RwLock<ToolPaths>>,
    data_dir: PathBuf,
    audio: Mutex<Slot>,
    video: Mutex<Slot>,
    generation: AtomicU64,
    events: mpsc::UnboundedSender<(EngineKind, u64, MpvEvent)>,
}

impl PlayerManager {
    pub fn new(db: Db, sink: Arc<dyn EventSink>, tools: Arc<RwLock<ToolPaths>>, data_dir: PathBuf) -> Arc<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        let manager = Arc::new(PlayerManager {
            settings: Settings::new(db.clone()),
            db,
            sink,
            tools,
            data_dir,
            audio: Mutex::new(Slot::new(EngineKind::Audio)),
            video: Mutex::new(Slot::new(EngineKind::Video)),
            generation: AtomicU64::new(1),
            events: tx,
        });
        tokio::spawn(Self::event_loop(Arc::downgrade(&manager), rx));
        manager
    }

    fn slot(&self, engine: EngineKind) -> &Mutex<Slot> {
        match engine {
            EngineKind::Audio => &self.audio,
            EngineKind::Video => &self.video,
        }
    }

    pub fn mpv_path(&self) -> Option<PathBuf> {
        self.tools.read().ok().and_then(|t| t.mpv.clone())
    }

    pub fn state(&self, engine: EngineKind) -> PlayerState {
        self.slot(engine).lock().unwrap().state.clone()
    }

    fn emit_state(&self, engine: EngineKind) {
        let state = {
            let mut slot = self.slot(engine).lock().unwrap();
            slot.last_emit = Instant::now();
            slot.state.clone()
        };
        self.sink.emit(CoreEvent::PlayerState(Box::new(state)));
    }

    fn process(&self, engine: EngineKind) -> Result<Arc<MpvProcess>> {
        self.slot(engine)
            .lock()
            .unwrap()
            .process
            .clone()
            .filter(|p| p.is_alive())
            .ok_or_else(|| CoreError::Player("Nothing is playing".into()))
    }

    fn input_conf(&self) -> Option<PathBuf> {
        let dir = self.data_dir.join("mpv");
        std::fs::create_dir_all(&dir).ok()?;
        let path = dir.join("input.conf");
        let conf = "# Nsembe Play key bindings (in addition to mpv defaults)\n\
                    N script-message nsembe-next\n\
                    P script-message nsembe-previous\n\
                    ESC set fullscreen no\n\
                    f cycle fullscreen\n";
        std::fs::write(&path, conf).ok()?;
        Some(path)
    }

    async fn ensure_process(&self, engine: EngineKind, geometry: Option<&str>) -> Result<Arc<MpvProcess>> {
        if let Some(p) = self.slot(engine).lock().unwrap().process.clone() {
            if p.is_alive() {
                return Ok(p);
            }
        }
        let mpv = self.mpv_path().ok_or_else(|| {
            CoreError::ToolUnavailable("mpv was not found. Install mpv or set its location in Settings → Playback.".into())
        })?;
        let volume = self.settings.get_f64(settings::VOLUME).await.unwrap_or(80.0).clamp(0.0, 100.0);
        let mut args = vec![format!("--volume={volume}")];
        match engine {
            EngineKind::Audio => {
                args.extend(
                    ["--video=no", "--audio-display=no", "--force-window=no", "--gapless-audio=weak", "--audio-client-name=Nsembe Play"]
                        .map(String::from),
                );
            }
            EngineKind::Video => {
                args.extend(
                    [
                        "--force-window=immediate",
                        "--title=${media-title} — Nsembe Play",
                        "--sub-auto=fuzzy",
                        "--keepaspect-window=yes",
                        "--cursor-autohide=1200",
                        "--osd-duration=1500",
                    ]
                    .map(String::from),
                );
                if let Some(conf) = self.input_conf() {
                    args.push(format!("--input-conf={}", conf.display()));
                }
                match geometry {
                    Some(g) if !g.is_empty() => args.push(format!("--geometry={g}")),
                    _ => args.push("--autofit=80%x80%".into()),
                }
                if self.settings.get_bool(settings::VIDEO_FULLSCREEN).await {
                    args.push("--fs".into());
                }
                let alang = self.settings.get_string(settings::AUDIO_LANGUAGE).await;
                if !alang.is_empty() {
                    args.push(format!("--alang={alang}"));
                }
                let slang = self.settings.get_string(settings::SUBTITLE_LANGUAGE).await;
                if !slang.is_empty() {
                    args.push(format!("--slang={slang}"));
                }
            }
        }

        let generation = self.generation.fetch_add(1, Ordering::SeqCst);
        let (tx, mut rx) = mpsc::unbounded_channel();
        let forward = self.events.clone();
        tokio::spawn(async move {
            while let Some(ev) = rx.recv().await {
                if forward.send((engine, generation, ev)).is_err() {
                    break;
                }
            }
        });
        let process = MpvProcess::spawn(&mpv, &args, tx).await?;
        for (i, name) in
            ["time-pos", "duration", "pause", "volume", "mute", "speed", "track-list", "fullscreen", "paused-for-cache"]
                .iter()
                .enumerate()
        {
            process.observe(i as u64 + 1, name).await?;
        }
        let mut slot = self.slot(engine).lock().unwrap();
        slot.process = Some(process.clone());
        slot.generation = generation;
        slot.state.active = true;
        slot.state.volume = volume;
        Ok(process)
    }

    /// Plays a media item with mpv. Music uses the audio engine, video the video engine.
    pub async fn play(&self, media_id: i64, options: PlayOptions) -> Result<PlayableItem> {
        let item = playback::playable(&self.db, media_id).await?;
        if !Path::new(&item.path).is_file() {
            media_db::set_missing(&self.db, media_id).await?;
            self.sink.emit(CoreEvent::LibraryChanged { reason: "missing".into() });
            return Err(CoreError::MissingFile(format!(
                "\"{}\" could not be found. It may have been moved, renamed or deleted.",
                item.title
            )));
        }
        let kind = MediaKind::parse(&item.kind).unwrap_or(MediaKind::Track);
        let engine = if kind.is_video() { EngineKind::Video } else { EngineKind::Audio };

        self.persist(engine, false).await;
        if engine == EngineKind::Video {
            // Starting a video pauses music.
            if let Ok(audio) = self.process(EngineKind::Audio) {
                let _ = audio.set_property("pause", json!(true)).await;
            }
        }
        let process = self.ensure_process(engine, options.geometry.as_deref()).await?;
        // Videos resume where the viewer stopped; songs always start from the top.
        let start = match options.start {
            Some(s) => s.max(0.0),
            None if kind.is_video() => playback::resume_position(&self.db, media_id).await?.unwrap_or(0.0),
            None => 0.0,
        };
        {
            let mut slot = self.slot(engine).lock().unwrap();
            slot.state.item = Some(item.clone());
            slot.state.position = start;
            slot.state.duration = item.duration;
            slot.state.paused = false;
            slot.state.audio_tracks.clear();
            slot.state.subtitle_tracks.clear();
            slot.loaded = false;
            slot.ended_handled = false;
            slot.last_persist = Instant::now();
        }
        let start_value = if start > 1.0 { json!(format!("{start:.3}")) } else { json!("none") };
        process.set_property("start", start_value).await?;
        process.set_property("force-media-title", json!(item.title.clone())).await?;
        process.command(vec![json!("loadfile"), json!(item.path.clone()), json!("replace")]).await?;
        process.set_property("pause", json!(false)).await?;
        playback::record_play_start(&self.db, media_id, item.duration).await?;
        self.emit_state(engine);
        Ok(item)
    }

    pub async fn command(&self, engine: EngineKind, cmd: PlayerCommand) -> Result<()> {
        if let PlayerCommand::Stop = cmd {
            return self.stop(engine).await;
        }
        let p = self.process(engine)?;
        match cmd {
            PlayerCommand::Pause => p.set_property("pause", json!(true)).await?,
            PlayerCommand::Resume => p.set_property("pause", json!(false)).await?,
            PlayerCommand::TogglePause => {
                p.command(vec![json!("cycle"), json!("pause")]).await?;
            }
            PlayerCommand::Seek { position } => {
                p.command(vec![json!("seek"), json!(position.max(0.0)), json!("absolute")]).await?;
            }
            PlayerCommand::SeekRelative { offset } => {
                p.command(vec![json!("seek"), json!(offset), json!("relative")]).await?;
            }
            PlayerCommand::SetVolume { volume } => {
                let v = volume.clamp(0.0, 100.0);
                p.set_property("volume", json!(v)).await?;
                let _ = self.settings.set(settings::VOLUME, json!(v.round())).await;
            }
            PlayerCommand::SetMuted { muted } => p.set_property("mute", json!(muted)).await?,
            PlayerCommand::SetSpeed { speed } => p.set_property("speed", json!(speed.clamp(0.25, 4.0))).await?,
            PlayerCommand::SetAudioTrack { id } => p.set_property("aid", json!(id)).await?,
            PlayerCommand::SetSubtitleTrack { id } => {
                p.set_property("sid", id.map(|i| json!(i)).unwrap_or(json!("no"))).await?
            }
            PlayerCommand::SetFullscreen { fullscreen } => p.set_property("fullscreen", json!(fullscreen)).await?,
            PlayerCommand::Stop => unreachable!(),
        }
        Ok(())
    }

    /// Stops playback: the audio engine goes idle, the video window closes.
    pub async fn stop(&self, engine: EngineKind) -> Result<()> {
        self.persist(engine, false).await;
        let process = self.slot(engine).lock().unwrap().process.clone();
        if let Some(p) = process {
            match engine {
                EngineKind::Audio => {
                    let _ = p.command(vec![json!("stop")]).await;
                }
                EngineKind::Video => p.quit().await,
            }
        }
        {
            let mut slot = self.slot(engine).lock().unwrap();
            slot.ended_handled = true;
            slot.state.item = None;
            slot.state.position = 0.0;
            slot.state.paused = true;
            if engine == EngineKind::Video {
                slot.process = None;
                slot.state.active = false;
            }
        }
        self.emit_state(engine);
        Ok(())
    }

    /// Saves the current position of an engine's item.
    async fn persist(&self, engine: EngineKind, ended: bool) {
        let snapshot = {
            let mut slot = self.slot(engine).lock().unwrap();
            slot.last_persist = Instant::now();
            if !slot.loaded && !ended {
                None
            } else {
                slot.state.item.as_ref().map(|i| (i.media_id, slot.state.position, slot.state.duration))
            }
        };
        if let Some((media_id, position, duration)) = snapshot {
            if let Err(e) = playback::save_progress(&self.db, media_id, position, duration, ended).await {
                log::error!("Saving playback position failed: {e}");
            }
        }
    }

    /// Next/previous episode relative to a media item (for the client's controls).
    pub async fn adjacent_episode(&self, media_id: i64, forward: bool) -> Result<Option<PlayableItem>> {
        let Some(current) = video::episode_by_media(&self.db, media_id).await? else { return Ok(None) };
        let ep = if forward {
            video::next_episode(&self.db, &current).await?
        } else {
            video::previous_episode(&self.db, &current).await?
        };
        match ep {
            Some(e) => Ok(Some(playback::playable(&self.db, e.media_id).await?)),
            None => Ok(None),
        }
    }

    /// Progress reported by a client-side player. Returns whether the item is now completed.
    pub async fn report_external(&self, media_id: i64, event: ExternalEvent, position: f64, duration: Option<f64>) -> Result<bool> {
        match event {
            ExternalEvent::Start => {
                playback::record_play_start(&self.db, media_id, duration).await?;
                Ok(false)
            }
            ExternalEvent::Ended => playback::save_progress(&self.db, media_id, position, duration, true).await,
            _ => playback::save_progress(&self.db, media_id, position, duration, false).await,
        }
    }

    /// Saves positions and terminates mpv processes (application exit).
    pub async fn shutdown(&self) {
        for engine in [EngineKind::Audio, EngineKind::Video] {
            self.persist(engine, false).await;
            let p = self.slot(engine).lock().unwrap().process.take();
            if let Some(p) = p {
                p.quit().await;
            }
        }
    }

    async fn event_loop(weak: std::sync::Weak<Self>, mut rx: mpsc::UnboundedReceiver<(EngineKind, u64, MpvEvent)>) {
        while let Some((engine, generation, event)) = rx.recv().await {
            let Some(this) = weak.upgrade() else { break };
            if this.slot(engine).lock().unwrap().generation != generation {
                continue; // from a process that has since been replaced
            }
            this.handle_event(engine, event).await;
        }
    }

    async fn handle_event(self: &Arc<Self>, engine: EngineKind, event: MpvEvent) {
        match event {
            MpvEvent::Property { name, data } => {
                let (emit, persist) = {
                    let mut slot = self.slot(engine).lock().unwrap();
                    let mut emit = true;
                    let mut persist = false;
                    match name.as_str() {
                        "time-pos" => {
                            if let (true, Some(t)) = (slot.loaded, data.as_f64()) {
                                slot.state.position = t;
                            }
                            emit = slot.last_emit.elapsed() >= EMIT_INTERVAL;
                            persist = slot.loaded && slot.last_persist.elapsed() >= PERSIST_INTERVAL;
                        }
                        "duration" => {
                            if let Some(d) = data.as_f64() {
                                slot.state.duration = Some(d);
                            }
                        }
                        "pause" => {
                            let paused = data.as_bool().unwrap_or(false);
                            slot.state.paused = paused;
                            persist = paused;
                        }
                        "volume" => slot.state.volume = data.as_f64().unwrap_or(slot.state.volume),
                        "mute" => slot.state.muted = data.as_bool().unwrap_or(false),
                        "speed" => slot.state.speed = data.as_f64().unwrap_or(1.0),
                        "fullscreen" => slot.state.fullscreen = data.as_bool().unwrap_or(false),
                        "paused-for-cache" => slot.state.buffering = data.as_bool().unwrap_or(false),
                        "track-list" => {
                            let (audio, subs) = parse_track_list(&data);
                            slot.state.audio_tracks = audio;
                            slot.state.subtitle_tracks = subs;
                        }
                        _ => emit = false,
                    }
                    (emit, persist)
                };
                if persist {
                    self.persist(engine, false).await;
                }
                if emit {
                    self.emit_state(engine);
                }
            }
            MpvEvent::FileLoaded => {
                let process = self.slot(engine).lock().unwrap().process.clone();
                if let Some(p) = process {
                    // `start` applies only to the file that requested it.
                    let _ = p.set_property("start", json!("none")).await;
                }
                self.slot(engine).lock().unwrap().loaded = true;
                self.emit_state(engine);
            }
            MpvEvent::EndFile { reason, error } => match reason.as_str() {
                "eof" => {
                    let first = {
                        let mut slot = self.slot(engine).lock().unwrap();
                        let first = !slot.ended_handled && slot.state.item.is_some();
                        slot.ended_handled = true;
                        if let Some(d) = slot.state.duration {
                            slot.state.position = d;
                        }
                        first
                    };
                    if first {
                        self.persist(engine, true).await;
                        let this = self.clone();
                        tokio::spawn(async move { this.on_finished(engine).await });
                    }
                }
                "error" => {
                    let item = {
                        let mut slot = self.slot(engine).lock().unwrap();
                        slot.ended_handled = true;
                        slot.state.item.clone()
                    };
                    let title = item.as_ref().map(|i| i.title.clone()).unwrap_or_default();
                    let message = format!(
                        "\"{title}\" could not be played. The file may be corrupted or in an unsupported format.{}",
                        error.map(|e| format!(" ({e})")).unwrap_or_default()
                    );
                    log::warn!("{message}");
                    self.sink.emit(Notice::error("Playback failed", message.clone()));
                    if let Some(i) = item {
                        self.sink.emit(CoreEvent::PlayerEnded(PlayerEnded {
                            engine,
                            media_id: i.media_id,
                            reason: "error".into(),
                            next_media_id: None,
                            message: Some(message),
                        }));
                    }
                }
                _ => {}
            },
            MpvEvent::ClientMessage(args) => {
                let current = self.slot(engine).lock().unwrap().state.item.as_ref().map(|i| i.media_id);
                if let (Some(media_id), Some(cmd)) = (current, args.first()) {
                    let forward = match cmd.as_str() {
                        "nsembe-next" => Some(true),
                        "nsembe-previous" => Some(false),
                        _ => None,
                    };
                    if let Some(forward) = forward {
                        if let Ok(Some(next)) = self.adjacent_episode(media_id, forward).await {
                            if let Err(e) = self.play(next.media_id, PlayOptions::default()).await {
                                self.sink.emit(Notice::error("Playback failed", e.to_string()));
                            }
                        }
                    }
                }
            }
            MpvEvent::Exited => {
                let (item, already_ended) = {
                    let slot = self.slot(engine).lock().unwrap();
                    (slot.state.item.clone(), slot.ended_handled)
                };
                if !already_ended {
                    self.persist(engine, false).await;
                }
                {
                    let mut slot = self.slot(engine).lock().unwrap();
                    slot.process = None;
                    slot.state = PlayerState { volume: slot.state.volume, ..PlayerState::idle(engine) };
                    slot.loaded = false;
                }
                self.emit_state(engine);
                if let Some(i) = item {
                    if !already_ended || engine == EngineKind::Video {
                        self.sink.emit(CoreEvent::PlayerEnded(PlayerEnded {
                            engine,
                            media_id: i.media_id,
                            reason: "closed".into(),
                            next_media_id: None,
                            message: None,
                        }));
                    }
                }
            }
        }
    }

    /// Natural end of an item: continue with the next episode or wrap up.
    async fn on_finished(self: Arc<Self>, engine: EngineKind) {
        let Some(item) = self.slot(engine).lock().unwrap().state.item.clone() else { return };
        let mut next_media_id = None;
        if engine == EngineKind::Video && item.kind == "episode" && self.settings.get_bool(settings::AUTOPLAY_NEXT_EPISODE).await {
            if let Ok(Some(next)) = self.adjacent_episode(item.media_id, true).await {
                match self.play(next.media_id, PlayOptions { start: Some(0.0), geometry: None }).await {
                    Ok(_) => next_media_id = Some(next.media_id),
                    Err(e) => log::warn!("Autoplay of next episode failed: {e}"),
                }
            }
        }
        self.sink.emit(CoreEvent::PlayerEnded(PlayerEnded {
            engine,
            media_id: item.media_id,
            reason: "eof".into(),
            next_media_id,
            message: None,
        }));
        if engine == EngineKind::Video && next_media_id.is_none() {
            // Nothing follows: close the video window.
            let _ = self.stop(EngineKind::Video).await;
        } else if engine == EngineKind::Audio {
            let mut slot = self.slot(engine).lock().unwrap();
            slot.state.paused = true;
        }
    }
}

fn parse_track_list(data: &Value) -> (Vec<TrackInfo>, Vec<TrackInfo>) {
    let mut audio = Vec::new();
    let mut subs = Vec::new();
    for t in data.as_array().into_iter().flatten() {
        let kind = t.get("type").and_then(|v| v.as_str()).unwrap_or_default();
        let info = TrackInfo {
            id: t.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
            kind: kind.to_string(),
            title: t.get("title").and_then(|v| v.as_str()).map(str::to_string),
            language: t.get("lang").and_then(|v| v.as_str()).map(str::to_string),
            codec: t.get("codec").and_then(|v| v.as_str()).map(str::to_string),
            selected: t.get("selected").and_then(|v| v.as_bool()).unwrap_or(false),
            external: t.get("external").and_then(|v| v.as_bool()).unwrap_or(false),
        };
        match kind {
            "audio" => audio.push(info),
            "sub" => subs.push(info),
            _ => {}
        }
    }
    (audio, subs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mpv_track_list() {
        let data = json!([
            {"id": 1, "type": "video", "selected": true},
            {"id": 1, "type": "audio", "lang": "eng", "codec": "aac", "selected": true},
            {"id": 2, "type": "audio", "lang": "jpn", "title": "Original"},
            {"id": 1, "type": "sub", "lang": "eng", "external": true}
        ]);
        let (audio, subs) = parse_track_list(&data);
        assert_eq!(audio.len(), 2);
        assert!(audio[0].selected);
        assert_eq!(audio[1].title.as_deref(), Some("Original"));
        assert_eq!(subs.len(), 1);
        assert!(subs[0].external);
    }

    #[test]
    fn deserializes_commands() {
        let cmd: PlayerCommand = serde_json::from_value(json!({"type": "seek", "position": 12.5})).unwrap();
        assert!(matches!(cmd, PlayerCommand::Seek { position } if position == 12.5));
        let cmd: PlayerCommand = serde_json::from_value(json!({"type": "setSubtitleTrack", "id": null})).unwrap();
        assert!(matches!(cmd, PlayerCommand::SetSubtitleTrack { id: None }));
    }
}
