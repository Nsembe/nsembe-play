// Audio engines for music playback. The player store talks to one of these:
//  - MpvAudioEngine: playback happens in Rust (mpv); state arrives via events.
//  - HtmlAudioEngine: a webview <audio> element used when mpv is unavailable.
// Both report history/progress to Rust so resume & "recently played" behave the same.

import { api } from "@/lib/api";
import { assetUrl } from "@/lib/media";
import type { EngineName, PlayableItem } from "@/types";

export interface EngineCallbacks {
  onTime: (position: number, duration: number | null) => void;
  onPaused: (paused: boolean) => void;
  onBuffering: (buffering: boolean) => void;
  onEnded: () => void;
  onError: (message: string) => void;
}

export interface AudioEngine {
  readonly name: EngineName;
  load(item: PlayableItem, start: number): Promise<void>;
  resume(): Promise<void>;
  pause(): Promise<void>;
  seek(position: number): Promise<void>;
  setVolume(volume: number): Promise<void>;
  setMuted(muted: boolean): Promise<void>;
  stop(): Promise<void>;
}

export class MpvAudioEngine implements AudioEngine {
  readonly name = "mpv" as const;

  async load(item: PlayableItem, start: number) {
    await api.play(item.mediaId, start);
  }
  async resume() {
    await api.playerCommand("audio", { type: "resume" });
  }
  async pause() {
    await api.playerCommand("audio", { type: "pause" });
  }
  async seek(position: number) {
    await api.playerCommand("audio", { type: "seek", position });
  }
  async setVolume(volume: number) {
    await api.playerCommand("audio", { type: "setVolume", volume }).catch(() => {});
  }
  async setMuted(muted: boolean) {
    await api.playerCommand("audio", { type: "setMuted", muted }).catch(() => {});
  }
  async stop() {
    await api.playerCommand("audio", { type: "stop" }).catch(() => {});
  }
}

const REPORT_INTERVAL_MS = 10_000;

export class HtmlAudioEngine implements AudioEngine {
  readonly name = "builtin" as const;
  private audio = new Audio();
  private current: PlayableItem | null = null;
  private lastReport = 0;

  constructor(cb: EngineCallbacks) {
    const a = this.audio;
    a.preload = "auto";
    a.addEventListener("timeupdate", () => {
      const dur = Number.isFinite(a.duration) ? a.duration : null;
      cb.onTime(a.currentTime, dur);
      if (this.current && Date.now() - this.lastReport > REPORT_INTERVAL_MS) {
        this.lastReport = Date.now();
        void api.report(this.current.mediaId, "progress", a.currentTime, dur).catch(() => {});
      }
    });
    a.addEventListener("play", () => cb.onPaused(false));
    a.addEventListener("pause", () => {
      cb.onPaused(true);
      this.reportNow("pause");
    });
    a.addEventListener("waiting", () => cb.onBuffering(true));
    a.addEventListener("playing", () => cb.onBuffering(false));
    a.addEventListener("ended", () => {
      this.reportNow("ended");
      cb.onEnded();
    });
    a.addEventListener("error", () => {
      if (!this.current) return;
      cb.onError(`"${this.current.title}" can't be played by the built-in player. Installing mpv adds support for more formats.`);
    });
  }

  private reportNow(event: "pause" | "stop" | "ended") {
    if (!this.current) return;
    const a = this.audio;
    const dur = Number.isFinite(a.duration) ? a.duration : this.current.duration;
    void api.report(this.current.mediaId, event, a.currentTime, dur).catch(() => {});
  }

  async load(item: PlayableItem, start: number) {
    if (this.current && !this.audio.paused) this.reportNow("stop");
    this.current = item;
    this.lastReport = Date.now();
    const url = assetUrl(item.path);
    if (!url) throw new Error("Missing file path");
    this.audio.src = url;
    if (start > 0) {
      this.audio.addEventListener("loadedmetadata", () => (this.audio.currentTime = start), { once: true });
    }
    void api.report(item.mediaId, "start", 0, item.duration).catch(() => {});
    await this.audio.play();
  }
  async resume() {
    await this.audio.play();
  }
  async pause() {
    this.audio.pause();
  }
  async seek(position: number) {
    if (Number.isFinite(position)) this.audio.currentTime = position;
  }
  async setVolume(volume: number) {
    this.audio.volume = Math.min(Math.max(volume / 100, 0), 1);
  }
  async setMuted(muted: boolean) {
    this.audio.muted = muted;
  }
  async stop() {
    this.reportNow("stop");
    this.audio.pause();
    this.audio.removeAttribute("src");
    this.audio.load();
    this.current = null;
  }
}
