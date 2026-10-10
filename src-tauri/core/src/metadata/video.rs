//! Video inspection through `ffprobe` (duration, resolution, codecs, audio and
//! subtitle streams, embedded tags).

use std::path::Path;
use std::time::Duration;

use serde_json::Value;
use tokio::process::Command;

use crate::database::media::TechInfo;
use crate::models::StreamInfo;
use crate::util::hide_console;

const PROBE_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, Default)]
pub struct VideoProbe {
    pub tech: TechInfo,
    pub title_tag: Option<String>,
    pub genre_tag: Option<String>,
    pub year_tag: Option<i64>,
    pub description_tag: Option<String>,
}

fn tag(v: &Value, key: &str) -> Option<String> {
    let tags = v.get("tags")?.as_object()?;
    tags.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .and_then(|(_, v)| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn num(v: &Value, key: &str) -> Option<f64> {
    match v.get(key)? {
        Value::String(s) => s.parse().ok(),
        Value::Number(n) => n.as_f64(),
        _ => None,
    }
}

/// Parses ffprobe's `-print_format json -show_format -show_streams` output.
pub fn parse_ffprobe_json(json: &Value) -> VideoProbe {
    let format = json.get("format").cloned().unwrap_or(Value::Null);
    let streams = json.get("streams").and_then(|s| s.as_array()).cloned().unwrap_or_default();
    let mut probe = VideoProbe::default();
    probe.tech.duration = num(&format, "duration").filter(|d| *d > 0.0);
    probe.tech.bitrate = num(&format, "bit_rate").map(|b| b as i64);
    probe.tech.container = format
        .get("format_name")
        .and_then(|v| v.as_str())
        .map(|s| s.split(',').next().unwrap_or(s).to_string());
    probe.title_tag = tag(&format, "title");
    probe.genre_tag = tag(&format, "genre");
    probe.description_tag = tag(&format, "description").or_else(|| tag(&format, "comment"));
    probe.year_tag = tag(&format, "date").or_else(|| tag(&format, "year")).and_then(|d| d.get(..4)?.parse().ok());

    for s in &streams {
        let codec_type = s.get("codec_type").and_then(|v| v.as_str()).unwrap_or("");
        let codec = s.get("codec_name").and_then(|v| v.as_str()).map(str::to_string);
        let disposition = s.get("disposition");
        let is_default = disposition.and_then(|d| d.get("default")).and_then(|v| v.as_i64()) == Some(1);
        let attached_pic = disposition.and_then(|d| d.get("attached_pic")).and_then(|v| v.as_i64()) == Some(1);
        let info = StreamInfo {
            index: s.get("index").and_then(|v| v.as_i64()).unwrap_or(0),
            codec: codec.clone(),
            language: tag(s, "language").filter(|l| l != "und"),
            title: tag(s, "title"),
            channels: s.get("channels").and_then(|v| v.as_i64()),
            default: is_default,
        };
        match codec_type {
            "video" if !attached_pic && probe.tech.video_codec.is_none() => {
                probe.tech.video_codec = codec;
                probe.tech.width = s.get("width").and_then(|v| v.as_i64());
                probe.tech.height = s.get("height").and_then(|v| v.as_i64());
                if probe.tech.duration.is_none() {
                    probe.tech.duration = num(s, "duration").filter(|d| *d > 0.0);
                }
            }
            "audio" => {
                if probe.tech.audio_codec.is_none() {
                    probe.tech.audio_codec = codec;
                }
                probe.tech.audio_streams.push(info);
            }
            "subtitle" => probe.tech.subtitle_streams.push(info),
            _ => {}
        }
    }
    probe
}

/// Runs ffprobe on a file. Errors are descriptive strings suitable for `scan_error`.
pub async fn probe_video(ffprobe: &Path, path: &Path) -> Result<VideoProbe, String> {
    let mut cmd = Command::new(ffprobe);
    cmd.args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams"])
        .arg(path)
        .kill_on_drop(true)
        .stdin(std::process::Stdio::null());
    hide_console(&mut cmd);
    let output = tokio::time::timeout(PROBE_TIMEOUT, cmd.output())
        .await
        .map_err(|_| "ffprobe timed out".to_string())?
        .map_err(|e| format!("Could not run ffprobe: {e}"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let first = err.lines().next().unwrap_or("unknown error").trim().to_string();
        return Err(format!("Unreadable or corrupted file ({first})"));
    }
    let json: Value =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("Invalid ffprobe output: {e}"))?;
    let probe = parse_ffprobe_json(&json);
    if probe.tech.video_codec.is_none() && probe.tech.audio_codec.is_none() {
        return Err("No playable streams found".to_string());
    }
    Ok(probe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_streams() {
        let json: Value = serde_json::from_str(
            r#"{
              "streams": [
                {"index":0,"codec_name":"hevc","codec_type":"video","width":3840,"height":2160,"disposition":{"default":1}},
                {"index":1,"codec_name":"eac3","codec_type":"audio","channels":6,"tags":{"language":"eng","title":"Surround"},"disposition":{"default":1}},
                {"index":2,"codec_name":"aac","codec_type":"audio","channels":2,"tags":{"language":"fre"}},
                {"index":3,"codec_name":"subrip","codec_type":"subtitle","tags":{"language":"eng"}},
                {"index":4,"codec_name":"mjpeg","codec_type":"video","disposition":{"attached_pic":1}}
              ],
              "format": {"format_name":"matroska,webm","duration":"7200.5","bit_rate":"15000000","tags":{"title":"Some Movie","DATE":"2019-05-01"}}
            }"#,
        )
        .unwrap();
        let p = parse_ffprobe_json(&json);
        assert_eq!(p.tech.duration, Some(7200.5));
        assert_eq!(p.tech.container.as_deref(), Some("matroska"));
        assert_eq!(p.tech.video_codec.as_deref(), Some("hevc"));
        assert_eq!((p.tech.width, p.tech.height), (Some(3840), Some(2160)));
        assert_eq!(p.tech.audio_streams.len(), 2);
        assert_eq!(p.tech.subtitle_streams.len(), 1);
        assert_eq!(p.year_tag, Some(2019));
        assert_eq!(p.tech.audio_streams[0].language.as_deref(), Some("eng"));
    }
}
