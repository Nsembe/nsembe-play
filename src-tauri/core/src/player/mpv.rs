//! A running mpv process controlled through its JSON IPC protocol
//! (named pipe on Windows, Unix socket elsewhere).
//!
//! Using the IPC protocol instead of linking libmpv keeps the build simple and
//! isolates crashes in malformed media from the application process.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot};

use crate::errors::{CoreError, Result};

/// Events coming from mpv.
#[derive(Debug, Clone)]
pub enum MpvEvent {
    Property { name: String, data: Value },
    FileLoaded,
    EndFile { reason: String, error: Option<String> },
    ClientMessage(Vec<String>),
    /// The process exited or the IPC connection closed.
    Exited,
}

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<std::result::Result<Value, String>>>>>;

static PIPE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct MpvProcess {
    writer: mpsc::UnboundedSender<String>,
    pending: Pending,
    next_id: AtomicU64,
    alive: Arc<AtomicBool>,
    child: tokio::sync::Mutex<Option<Child>>,
}

fn ipc_path() -> String {
    let n = PIPE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    if cfg!(windows) {
        format!(r"\\.\pipe\nsembe-play-mpv-{pid}-{n}")
    } else {
        std::env::temp_dir().join(format!("nsembe-play-mpv-{pid}-{n}.sock")).to_string_lossy().into_owned()
    }
}

#[cfg(windows)]
async fn connect(path: &str) -> std::io::Result<(Box<dyn AsyncRead + Send + Unpin>, Box<dyn AsyncWrite + Send + Unpin>)> {
    use tokio::net::windows::named_pipe::ClientOptions;
    let client = ClientOptions::new().open(path)?;
    let (r, w) = tokio::io::split(client);
    Ok((Box::new(r), Box::new(w)))
}

#[cfg(unix)]
async fn connect(path: &str) -> std::io::Result<(Box<dyn AsyncRead + Send + Unpin>, Box<dyn AsyncWrite + Send + Unpin>)> {
    let stream = tokio::net::UnixStream::connect(path).await?;
    let (r, w) = stream.into_split();
    Ok((Box::new(r), Box::new(w)))
}

impl MpvProcess {
    /// Starts mpv and connects to its IPC server. Events are delivered on `events`.
    pub async fn spawn(mpv: &Path, extra_args: &[String], events: mpsc::UnboundedSender<MpvEvent>) -> Result<Arc<Self>> {
        let ipc = ipc_path();
        let mut cmd = Command::new(mpv);
        cmd.arg(format!("--input-ipc-server={ipc}"))
            .args(["--idle=yes", "--no-terminal", "--config=no", "--keep-open=no", "--hr-seek=yes"])
            .args(["--ytdl=no", "--save-position-on-quit=no", "--resume-playback=no", "--volume-max=100"])
            .args(extra_args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        crate::util::hide_console(&mut cmd);
        let mut child = cmd
            .spawn()
            .map_err(|e| CoreError::Player(format!("Could not start mpv ({}): {e}", mpv.display())))?;

        // mpv needs a moment to create the IPC endpoint.
        let mut connection = None;
        for _ in 0..100 {
            if let Ok(Some(status)) = child.try_wait() {
                return Err(CoreError::Player(format!("mpv exited immediately ({status})")));
            }
            match connect(&ipc).await {
                Ok(c) => {
                    connection = Some(c);
                    break;
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        }
        let Some((reader, mut writer)) = connection else {
            let _ = child.kill().await;
            return Err(CoreError::Player("Timed out connecting to mpv".into()));
        };

        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let alive = Arc::new(AtomicBool::new(true));
        let (wtx, mut wrx) = mpsc::unbounded_channel::<String>();

        tokio::spawn(async move {
            while let Some(line) = wrx.recv().await {
                if writer.write_all(line.as_bytes()).await.is_err() || writer.write_all(b"\n").await.is_err() {
                    break;
                }
                let _ = writer.flush().await;
            }
        });

        {
            let pending = pending.clone();
            let alive = alive.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(reader).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
                    if let Some(event) = msg.get("event").and_then(|e| e.as_str()) {
                        let ev = match event {
                            "property-change" => MpvEvent::Property {
                                name: msg.get("name").and_then(|n| n.as_str()).unwrap_or_default().to_string(),
                                data: msg.get("data").cloned().unwrap_or(Value::Null),
                            },
                            "file-loaded" => MpvEvent::FileLoaded,
                            "end-file" => MpvEvent::EndFile {
                                reason: msg.get("reason").and_then(|r| r.as_str()).unwrap_or("unknown").to_string(),
                                error: msg.get("file_error").and_then(|r| r.as_str()).map(str::to_string),
                            },
                            "client-message" => MpvEvent::ClientMessage(
                                msg.get("args")
                                    .and_then(|a| a.as_array())
                                    .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                                    .unwrap_or_default(),
                            ),
                            _ => continue,
                        };
                        let _ = events.send(ev);
                    } else if let Some(id) = msg.get("request_id").and_then(|r| r.as_u64()) {
                        if let Some(tx) = pending.lock().unwrap().remove(&id) {
                            let status = msg.get("error").and_then(|e| e.as_str()).unwrap_or("success");
                            let result = if status == "success" {
                                Ok(msg.get("data").cloned().unwrap_or(Value::Null))
                            } else {
                                Err(status.to_string())
                            };
                            let _ = tx.send(result);
                        }
                    }
                }
                alive.store(false, Ordering::SeqCst);
                for (_, tx) in pending.lock().unwrap().drain() {
                    let _ = tx.send(Err("mpv exited".into()));
                }
                let _ = events.send(MpvEvent::Exited);
            });
        }

        Ok(Arc::new(MpvProcess {
            writer: wtx,
            pending,
            next_id: AtomicU64::new(1),
            alive,
            child: tokio::sync::Mutex::new(Some(child)),
        }))
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    /// Sends a command and waits for mpv's reply.
    pub async fn command(&self, args: Vec<Value>) -> Result<Value> {
        if !self.is_alive() {
            return Err(CoreError::Player("Player is not running".into()));
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);
        let line = json!({ "command": args, "request_id": id }).to_string();
        if self.writer.send(line).is_err() {
            self.pending.lock().unwrap().remove(&id);
            return Err(CoreError::Player("Player connection closed".into()));
        }
        match tokio::time::timeout(Duration::from_secs(10), rx).await {
            Ok(Ok(Ok(v))) => Ok(v),
            Ok(Ok(Err(e))) => Err(CoreError::Player(format!("mpv: {e}"))),
            Ok(Err(_)) => Err(CoreError::Player("Player connection closed".into())),
            Err(_) => {
                self.pending.lock().unwrap().remove(&id);
                Err(CoreError::Player("mpv did not respond".into()))
            }
        }
    }

    pub async fn set_property(&self, name: &str, value: Value) -> Result<()> {
        self.command(vec![json!("set_property"), json!(name), value]).await.map(|_| ())
    }

    pub async fn observe(&self, id: u64, name: &str) -> Result<()> {
        self.command(vec![json!("observe_property"), json!(id), json!(name)]).await.map(|_| ())
    }

    /// Asks mpv to quit, killing it if it does not exit promptly.
    pub async fn quit(&self) {
        let _ = tokio::time::timeout(Duration::from_secs(2), self.command(vec![json!("quit")])).await;
        let mut guard = self.child.lock().await;
        if let Some(mut child) = guard.take() {
            match tokio::time::timeout(Duration::from_secs(2), child.wait()).await {
                Ok(_) => {}
                Err(_) => {
                    let _ = child.kill().await;
                }
            }
        }
    }
}
