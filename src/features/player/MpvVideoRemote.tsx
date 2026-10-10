// Companion screen while a video plays in mpv's window: mirrors its state and
// offers full controls (tracks, speed, episodes) from inside the app.

import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router";
import { ArrowLeft, Maximize, MonitorPlay, Pause, Play, RotateCcw, RotateCw, SkipBack, SkipForward, Square, Volume2, VolumeX } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { Artwork } from "@/components/Artwork";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { Tip } from "@/components/ui/misc";
import { episodeParts } from "@/features/library/cards";
import { api } from "@/lib/api";
import { assetUrl } from "@/lib/media";
import { formatTime } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";
import type { PlayerCommand } from "@/types";
import { SpeedMenu, TrackMenu, languageLabel, videoButtonClass } from "./VideoControls";

const send = (command: PlayerCommand) => void api.playerCommand("video", command).catch(() => {});

export function MpvVideoRemote({ mediaId }: { mediaId: number }) {
  const navigate = useNavigate();
  const state = usePlayerStore((s) => s.video);
  const wasActive = useRef(false);
  const [drag, setDrag] = useState<number | null>(null);

  // Pick up the current state in case events fired before this page mounted.
  useEffect(() => {
    void api.playerState("video").then((s) => usePlayerStore.getState().applyMpvVideo(s));
  }, [mediaId]);

  // When the mpv window closes, return to where the user came from.
  useEffect(() => {
    if (state?.active) wasActive.current = true;
    else if (wasActive.current && state && !state.active) navigate(-1);
  }, [state, navigate]);

  const item = state?.item;
  const currentId = item?.mediaId ?? mediaId;
  const { data: next } = useQuery({
    queryKey: ["adjacent", currentId, true],
    queryFn: () => api.adjacentEpisode(currentId, true),
    enabled: item?.kind === "episode",
  });
  const { data: prev } = useQuery({
    queryKey: ["adjacent", currentId, false],
    queryFn: () => api.adjacentEpisode(currentId, false),
    enabled: item?.kind === "episode",
  });

  if (!state || !item) {
    return (
      <div className="grid h-full place-items-center">
        <div className="text-center">
          <MonitorPlay className="mx-auto size-12 text-muted" />
          <p className="mt-4 text-muted">Starting playback…</p>
          <Button variant="ghost" className="mt-6" onClick={() => navigate(-1)}>
            <ArrowLeft />
            Back
          </Button>
        </div>
      </div>
    );
  }

  const isEpisode = item.kind === "episode";
  const { show, code } = episodeParts(item);
  const duration = state.duration ?? item.duration ?? 0;
  const shown = drag ?? state.position;
  const bg = assetUrl(item.backdrop ?? item.artwork);
  const play = (id: number) => void api.play(id, 0).then(() => navigate(`/watch/${id}`, { replace: true }));

  return (
    <div className="relative h-full overflow-hidden">
      {bg && <img src={bg} alt="" className="absolute inset-0 size-full scale-110 object-cover opacity-30 blur-2xl" />}
      <div className="absolute inset-0 bg-gradient-to-t from-bg via-bg/80 to-bg/50" />

      <div className="relative flex h-full flex-col px-10 py-8">
        <div className="flex items-center gap-3">
          <Tip label="Back to library">
            <button type="button" onClick={() => navigate(-1)} className="grid size-10 place-items-center rounded-full bg-black/30 hover:bg-black/50">
              <ArrowLeft className="size-5" />
            </button>
          </Tip>
          <div className="flex items-center gap-2 rounded-full bg-accent/15 px-3 py-1.5 text-[12.5px] font-semibold text-accent">
            <MonitorPlay className="size-4" />
            Playing in the video window
          </div>
        </div>

        <div className="flex flex-1 items-center gap-10">
          <Artwork
            src={isEpisode ? (item.artwork ?? item.backdrop) : (item.artwork ?? item.backdrop)}
            title={item.title}
            kind={isEpisode ? "episode" : "movie"}
            showTitle
            className={isEpisode ? "aspect-video w-[min(40vw,520px)] rounded-xl shadow-pop" : "aspect-[2/3] w-[min(22vw,260px)] rounded-xl shadow-pop"}
          />
          <div className="max-w-xl min-w-0">
            <h1 className="font-display text-[clamp(2rem,3.6vw,3rem)] leading-tight font-extrabold tracking-tight text-balance">{isEpisode ? show : item.title}</h1>
            {isEpisode && (
              <p className="mt-2 text-[16px] text-fg/80">
                {code} · {item.title}
              </p>
            )}
            <p className="mt-4 text-[13.5px] text-muted">
              Use the controls below or the mpv window itself (Space, arrows, F for fullscreen{isEpisode ? ", N/P for next/previous episode" : ""}). Your
              position is saved automatically.
            </p>
          </div>
        </div>

        <div className="rounded-2xl bg-black/30 p-5 backdrop-blur-xl">
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
                send({ type: "seek", position: v });
              }}
            />
            <span className="w-14">{formatTime(duration)}</span>
          </div>
          <div className="mt-3 flex items-center gap-1">
            {prev && (
              <Tip label="Previous episode">
                <button type="button" onClick={() => play(prev.mediaId)} className={videoButtonClass}>
                  <SkipBack className="size-5 fill-current" />
                </button>
              </Tip>
            )}
            <Tip label={state.paused ? "Play" : "Pause"}>
              <button type="button" onClick={() => send({ type: "togglePause" })} className="grid size-12 place-items-center rounded-full bg-fg text-bg transition hover:scale-105">
                {state.paused ? <Play className="size-6 translate-x-px fill-current" /> : <Pause className="size-6 fill-current" />}
              </button>
            </Tip>
            {next && (
              <Tip label="Next episode">
                <button type="button" onClick={() => play(next.mediaId)} className={videoButtonClass}>
                  <SkipForward className="size-5 fill-current" />
                </button>
              </Tip>
            )}
            <Tip label="Back 10 seconds">
              <button type="button" onClick={() => send({ type: "seekRelative", offset: -10 })} className={videoButtonClass}>
                <RotateCcw className="size-5" />
              </button>
            </Tip>
            <Tip label="Forward 10 seconds">
              <button type="button" onClick={() => send({ type: "seekRelative", offset: 10 })} className={videoButtonClass}>
                <RotateCw className="size-5" />
              </button>
            </Tip>
            <div className="ml-2 flex items-center gap-1">
              <button type="button" onClick={() => send({ type: "setMuted", muted: !state.muted })} className={videoButtonClass} aria-label="Mute">
                {state.muted ? <VolumeX className="size-5" /> : <Volume2 className="size-5" />}
              </button>
              <Slider
                aria-label="Volume"
                className="w-28"
                min={0}
                max={100}
                step={1}
                value={[state.volume]}
                onValueChange={([v]) => send({ type: "setVolume", volume: v })}
              />
            </div>
            <div className="ml-auto flex items-center gap-1">
              <TrackMenu
                kind="audio"
                value={String(state.audioTracks.find((t) => t.selected)?.id ?? "")}
                onChange={(id) => send({ type: "setAudioTrack", id: Number(id) })}
                options={state.audioTracks.map((t) => ({ id: String(t.id), label: languageLabel(t.language, t.title, `Track ${t.id}`) }))}
              />
              <TrackMenu
                kind="subtitles"
                allowOff
                value={String(state.subtitleTracks.find((t) => t.selected)?.id ?? "off")}
                onChange={(id) => send({ type: "setSubtitleTrack", id: id === "off" ? null : Number(id) })}
                options={state.subtitleTracks.map((t) => ({
                  id: String(t.id),
                  label: languageLabel(t.language, t.title, `Subtitle ${t.id}`) + (t.external ? " (file)" : ""),
                }))}
              />
              <SpeedMenu speed={state.speed} onChange={(s) => send({ type: "setSpeed", speed: s })} />
              <Tip label="Fullscreen">
                <button type="button" onClick={() => send({ type: "setFullscreen", fullscreen: !state.fullscreen })} className={videoButtonClass}>
                  <Maximize className="size-5" />
                </button>
              </Tip>
              <Tip label="Stop">
                <button type="button" onClick={() => send({ type: "stop" })} className={videoButtonClass}>
                  <Square className="size-4 fill-current" />
                </button>
              </Tip>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
