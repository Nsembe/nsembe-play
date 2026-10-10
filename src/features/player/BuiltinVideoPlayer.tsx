// In-app video player used when mpv is not available (or not selected).
// Plays through the webview's <video> element via the asset protocol, scoped to
// library folders. Progress is reported to Rust for resume/continue watching.

import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  AlertTriangle,
  ArrowLeft,
  FolderOpen,
  Loader2,
  Maximize,
  Minimize,
  Pause,
  Play,
  RotateCcw,
  RotateCw,
  Settings as SettingsIcon,
  SkipBack,
  SkipForward,
  Volume2,
  VolumeX,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { Tip } from "@/components/ui/misc";
import { api } from "@/lib/api";
import { assetUrl } from "@/lib/media";
import { cn, formatTime, isTypingTarget } from "@/lib/utils";
import { useUiStore } from "@/stores/uiStore";
import type { PlayableItem } from "@/types";
import { SpeedMenu, TrackMenu, videoButtonClass } from "./VideoControls";
import { episodeParts } from "@/features/library/cards";

const REPORT_MS = 10_000;

/** Window fullscreen must never break the player (e.g. permission or platform errors). */
async function setWindowFullscreen(on: boolean) {
  try {
    await getCurrentWindow().setFullscreen(on);
  } catch {
    // Keep the in-app immersive mode even if the window can't change.
  }
}
const VOLUME_KEY = "nsembe-video-volume";

function UpNext({ next, onPlay, onCancel }: { next: PlayableItem; onPlay: () => void; onCancel: () => void }) {
  const [left, setLeft] = useState(10);
  useEffect(() => {
    if (left <= 0) {
      onPlay();
      return;
    }
    const t = setTimeout(() => setLeft((l) => l - 1), 1000);
    return () => clearTimeout(t);
  }, [left, onPlay]);
  const { code } = episodeParts(next);
  return (
    <div className="absolute right-8 bottom-28 z-20 w-[340px] animate-rise rounded-2xl bg-surface-2/95 p-5 shadow-pop backdrop-blur-xl">
      <div className="text-[12px] font-bold tracking-[0.16em] text-accent uppercase">Up next in {left}s</div>
      <div className="mt-2 text-[15px] font-semibold">{next.title}</div>
      <div className="text-[13px] text-muted">{code}</div>
      <div className="mt-4 flex gap-2">
        <Button variant="primary" size="sm" onClick={onPlay}>
          <Play className="fill-current" />
          Play now
        </Button>
        <Button variant="ghost" size="sm" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}

export function BuiltinVideoPlayer({ mediaId, start }: { mediaId: number; start: number | null }) {
  const navigate = useNavigate();
  const videoRef = useRef<HTMLVideoElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const lastReport = useRef(0);
  const started = useRef(false);
  // Last known position: the <video> ref is already detached when unmount cleanup runs.
  const lastPos = useRef<{ time: number; duration: number | null }>({ time: 0, duration: null });
  const hideTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const setImmersive = useUiStore((s) => s.setImmersive);

  const [playing, setPlaying] = useState(false);
  const [buffering, setBuffering] = useState(true);
  const [time, setTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [drag, setDrag] = useState<number | null>(null);
  const [volume, setVolume] = useState(() => Number(localStorage.getItem(VOLUME_KEY) ?? 1));
  const [muted, setMuted] = useState(false);
  const [speed, setSpeed] = useState(1);
  const [fullscreen, setFullscreen] = useState(false);
  const [controls, setControls] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [subtitle, setSubtitle] = useState("off");
  const [ended, setEnded] = useState(false);

  const { data: item, error: resolveError } = useQuery({
    queryKey: ["resolve", mediaId],
    queryFn: () => api.resolve(mediaId),
    staleTime: Infinity,
    gcTime: 0,
  });
  const { data: subs } = useQuery({ queryKey: ["subtitles", mediaId], queryFn: () => api.subtitles(mediaId), staleTime: Infinity });
  const { data: next } = useQuery({
    queryKey: ["adjacent", mediaId, true],
    queryFn: () => api.adjacentEpisode(mediaId, true),
    enabled: item?.kind === "episode",
  });
  const { data: prev } = useQuery({
    queryKey: ["adjacent", mediaId, false],
    queryFn: () => api.adjacentEpisode(mediaId, false),
    enabled: item?.kind === "episode",
  });

  const [subUrls, setSubUrls] = useState<string[]>([]);
  useEffect(() => {
    const urls = (subs ?? []).map((s) => URL.createObjectURL(new Blob([s.vtt], { type: "text/vtt" })));
    setSubUrls(urls);
    return () => urls.forEach((u) => URL.revokeObjectURL(u));
  }, [subs]);

  useEffect(() => {
    const v = videoRef.current;
    if (!v) return;
    Array.from(v.textTracks).forEach((t, i) => (t.mode = subtitle === String(i) ? "showing" : "disabled"));
  }, [subtitle, subUrls]);

  const report = useCallback(
    (event: "start" | "progress" | "pause" | "stop" | "ended") => {
      if (!item) return;
      const v = videoRef.current;
      if (v) lastPos.current = { time: v.currentTime, duration: Number.isFinite(v.duration) ? v.duration : item.duration };
      const { time, duration } = lastPos.current;
      void api.report(item.mediaId, event, time, duration ?? item.duration).catch(() => {});
    },
    [item],
  );

  // Save the position when leaving the player.
  useEffect(() => {
    return () => {
      if (started.current) report("stop");
    };
  }, [report]);

  // Leave fullscreen/immersive mode when unmounting.
  useEffect(() => {
    return () => {
      setImmersive(false);
      void setWindowFullscreen(false);
    };
  }, [setImmersive]);

  const toggleFullscreen = useCallback(async () => {
    const next = !fullscreen;
    setFullscreen(next);
    setImmersive(next);
    await setWindowFullscreen(next);
  }, [fullscreen, setImmersive]);

  const togglePlay = useCallback(() => {
    const v = videoRef.current;
    if (!v) return;
    if (v.paused) void v.play();
    else v.pause();
  }, []);

  const seekBy = useCallback((delta: number) => {
    const v = videoRef.current;
    if (v) v.currentTime = Math.min(Math.max(v.currentTime + delta, 0), v.duration || Infinity);
  }, []);

  const goTo = useCallback((target: PlayableItem) => navigate(`/watch/${target.mediaId}?t=0`, { replace: true }), [navigate]);

  const poke = useCallback(() => {
    setControls(true);
    clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => {
      if (!videoRef.current?.paused) setControls(false);
    }, 2600);
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (isTypingTarget(e.target)) return;
      const v = videoRef.current;
      if (!v) return;
      poke();
      switch (e.key) {
        case " ":
        case "k":
          e.preventDefault();
          togglePlay();
          break;
        case "ArrowRight":
          seekBy(10);
          break;
        case "ArrowLeft":
          seekBy(-10);
          break;
        case "ArrowUp":
          e.preventDefault();
          v.volume = Math.min(v.volume + 0.05, 1);
          break;
        case "ArrowDown":
          e.preventDefault();
          v.volume = Math.max(v.volume - 0.05, 0);
          break;
        case "f":
          void toggleFullscreen();
          break;
        case "m":
          v.muted = !v.muted;
          break;
        case "N":
          if (next) goTo(next);
          break;
        case "P":
          if (prev) goTo(prev);
          break;
        case "Escape":
          if (fullscreen) void toggleFullscreen();
          else navigate(-1);
          break;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [togglePlay, seekBy, toggleFullscreen, fullscreen, navigate, next, prev, goTo, poke]);

  const failure = error ?? (resolveError ? (resolveError as Error).message : null);
  const title = item ? (item.kind === "episode" ? episodeParts(item).show : item.title) : "";
  const subtitleLine = item?.kind === "episode" ? `${episodeParts(item).code} · ${item.title}` : null;
  const shown = drag ?? time;

  return (
    <div
      ref={containerRef}
      onMouseMove={poke}
      className={cn("relative h-full w-full overflow-hidden bg-black", !controls && playing && "cursor-none")}
    >
      {item && !failure && (
        <video
          key={item.mediaId}
          ref={videoRef}
          src={assetUrl(item.path)}
          autoPlay
          className="absolute inset-0 size-full object-contain"
          onClick={togglePlay}
          onDoubleClick={() => void toggleFullscreen()}
          onLoadedMetadata={(e) => {
            const v = e.currentTarget;
            v.volume = volume;
            const resumeAt = start ?? item.position ?? 0;
            if (resumeAt > 0 && resumeAt < v.duration - 5) v.currentTime = resumeAt;
            setDuration(v.duration);
          }}
          onPlay={() => {
            setPlaying(true);
            setEnded(false);
            if (!started.current) {
              started.current = true;
              report("start");
            }
            poke();
          }}
          onPause={() => {
            setPlaying(false);
            setControls(true);
            report("pause");
          }}
          onWaiting={() => setBuffering(true)}
          onPlaying={() => setBuffering(false)}
          onCanPlay={() => setBuffering(false)}
          onTimeUpdate={(e) => {
            const v = e.currentTarget;
            setTime(v.currentTime);
            lastPos.current = { time: v.currentTime, duration: Number.isFinite(v.duration) ? v.duration : null };
            if (Date.now() - lastReport.current > REPORT_MS) {
              lastReport.current = Date.now();
              report("progress");
            }
          }}
          onVolumeChange={(e) => {
            setVolume(e.currentTarget.volume);
            setMuted(e.currentTarget.muted);
            localStorage.setItem(VOLUME_KEY, String(e.currentTarget.volume));
          }}
          onRateChange={(e) => setSpeed(e.currentTarget.playbackRate)}
          onEnded={() => {
            report("ended");
            setEnded(true);
            setControls(true);
          }}
          onError={() =>
            setError("This video's format isn't supported by the built-in player. Install mpv (Settings → Media tools) to play MKV, HEVC and other formats.")
          }
        >
          {subs?.map((s, i) => (
            <track key={i} kind="subtitles" src={subUrls[i]} label={s.label} srcLang={s.language ?? undefined} />
          ))}
        </video>
      )}

      {buffering && !failure && item && (
        <div className="pointer-events-none absolute inset-0 grid place-items-center">
          <Loader2 className="size-12 animate-spin text-white/70" />
        </div>
      )}

      {failure && (
        <div className="absolute inset-0 grid place-items-center p-8">
          <div className="max-w-md text-center">
            <AlertTriangle className="mx-auto size-12 text-warning" />
            <h2 className="mt-4 font-display text-2xl font-bold">Can't play this video</h2>
            <p className="mt-2 text-[14px] text-muted">{failure}</p>
            <div className="mt-6 flex justify-center gap-2">
              <Button variant="secondary" onClick={() => navigate(-1)}>
                <ArrowLeft />
                Go back
              </Button>
              <Button variant="secondary" onClick={() => navigate("/settings")}>
                <SettingsIcon />
                Settings
              </Button>
              <Button variant="ghost" onClick={() => void api.reveal(mediaId)}>
                <FolderOpen />
                Show in folder
              </Button>
            </div>
          </div>
        </div>
      )}

      {ended && next && <UpNext next={next} onPlay={() => goTo(next)} onCancel={() => setEnded(false)} />}

      {/* Top bar */}
      <div
        className={cn(
          "absolute inset-x-0 top-0 flex items-center gap-4 bg-gradient-to-b from-black/80 to-transparent px-6 pt-5 pb-14 transition-opacity duration-300",
          controls || !playing ? "opacity-100" : "pointer-events-none opacity-0",
        )}
      >
        <Tip label="Back">
          <button type="button" onClick={() => navigate(-1)} className={videoButtonClass}>
            <ArrowLeft className="size-5" />
          </button>
        </Tip>
        <div className="min-w-0">
          <div className="truncate font-display text-lg font-bold">{title}</div>
          {subtitleLine && <div className="truncate text-[13px] text-white/70">{subtitleLine}</div>}
        </div>
      </div>

      {/* Bottom controls */}
      {!failure && (
        <div
          className={cn(
            "absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/90 via-black/50 to-transparent px-6 pt-16 pb-5 transition-opacity duration-300",
            controls || !playing ? "opacity-100" : "pointer-events-none opacity-0",
          )}
        >
          <div className="flex items-center gap-3 text-[12.5px] text-white/80 tabular-nums">
            <span className="w-14 text-right">{formatTime(shown)}</span>
            <Slider
              aria-label="Seek"
              min={0}
              max={Math.max(duration, 1)}
              step={1}
              value={[Math.min(shown, Math.max(duration, 1))]}
              onValueChange={([v]) => setDrag(v)}
              onValueCommit={([v]) => {
                setDrag(null);
                if (videoRef.current) videoRef.current.currentTime = v;
              }}
            />
            <span className="w-14">-{formatTime(Math.max(duration - shown, 0))}</span>
          </div>
          <div className="mt-2 flex items-center gap-1">
            {prev && (
              <Tip label="Previous episode">
                <button type="button" onClick={() => goTo(prev)} className={videoButtonClass}>
                  <SkipBack className="size-5 fill-current" />
                </button>
              </Tip>
            )}
            <Tip label={playing ? "Pause" : "Play"}>
              <button type="button" onClick={togglePlay} className="grid size-12 place-items-center rounded-full text-white transition hover:bg-white/10">
                {playing ? <Pause className="size-7 fill-current" /> : <Play className="size-7 translate-x-px fill-current" />}
              </button>
            </Tip>
            {next && (
              <Tip label="Next episode">
                <button type="button" onClick={() => goTo(next)} className={videoButtonClass}>
                  <SkipForward className="size-5 fill-current" />
                </button>
              </Tip>
            )}
            <Tip label="Back 10 seconds">
              <button type="button" onClick={() => seekBy(-10)} className={videoButtonClass}>
                <RotateCcw className="size-5" />
              </button>
            </Tip>
            <Tip label="Forward 10 seconds">
              <button type="button" onClick={() => seekBy(10)} className={videoButtonClass}>
                <RotateCw className="size-5" />
              </button>
            </Tip>
            <div className="ml-2 flex items-center gap-1">
              <button
                type="button"
                onClick={() => videoRef.current && (videoRef.current.muted = !videoRef.current.muted)}
                className={videoButtonClass}
                aria-label={muted ? "Unmute" : "Mute"}
              >
                {muted || volume === 0 ? <VolumeX className="size-5" /> : <Volume2 className="size-5" />}
              </button>
              <Slider
                aria-label="Volume"
                className="w-24"
                min={0}
                max={1}
                step={0.01}
                value={[muted ? 0 : volume]}
                onValueChange={([v]) => {
                  if (videoRef.current) {
                    videoRef.current.volume = v;
                    videoRef.current.muted = false;
                  }
                }}
              />
            </div>
            <div className="ml-auto flex items-center gap-1">
              <TrackMenu
                kind="subtitles"
                allowOff
                value={subtitle}
                onChange={setSubtitle}
                options={(subs ?? []).map((s, i) => ({ id: String(i), label: s.label }))}
              />
              <SpeedMenu
                speed={speed}
                onChange={(s) => {
                  if (videoRef.current) videoRef.current.playbackRate = s;
                }}
              />
              <Tip label={fullscreen ? "Exit fullscreen" : "Fullscreen"}>
                <button type="button" onClick={() => void toggleFullscreen()} className={videoButtonClass}>
                  {fullscreen ? <Minimize className="size-5" /> : <Maximize className="size-5" />}
                </button>
              </Tip>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
