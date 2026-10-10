import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** 75 -> "1:15", 4000 -> "1:06:40" */
export function formatTime(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) return "0:00";
  const s = Math.floor(seconds);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const pad = (n: number) => n.toString().padStart(2, "0");
  return h > 0 ? `${h}:${pad(m)}:${pad(sec)}` : `${m}:${pad(sec)}`;
}

/** 8100 -> "2h 15m", 300 -> "5m" */
export function formatDuration(seconds: number | null | undefined): string {
  if (!seconds || !Number.isFinite(seconds) || seconds <= 0) return "";
  const totalMin = Math.round(seconds / 60);
  const h = Math.floor(totalMin / 60);
  const m = totalMin % 60;
  if (h === 0) return `${Math.max(m, 1)}m`;
  return m === 0 ? `${h}h` : `${h}h ${m}m`;
}

/** Long durations for collections: "1 hr 12 min" / "48 min". */
export function formatLongDuration(seconds: number | null | undefined): string {
  if (!seconds || seconds <= 0) return "";
  const totalMin = Math.round(seconds / 60);
  const h = Math.floor(totalMin / 60);
  const m = totalMin % 60;
  if (h === 0) return `${m} min`;
  return m === 0 ? `${h} hr` : `${h} hr ${m} min`;
}

export function remainingLabel(position: number | null | undefined, duration: number | null | undefined): string {
  if (!position || !duration) return "";
  const left = Math.max(duration - position, 0);
  if (left < 60) return "Almost done";
  return `${formatDuration(left)} left`;
}

export function watchedLabel(position: number | null | undefined): string {
  if (!position) return "";
  return `${formatDuration(position)} watched`;
}

export function progressFraction(position: number | null | undefined, duration: number | null | undefined): number {
  if (!position || !duration || duration <= 0) return 0;
  return Math.min(Math.max(position / duration, 0), 1);
}

export function plural(n: number, word: string, pluralWord?: string): string {
  return `${n.toLocaleString()} ${n === 1 ? word : (pluralWord ?? `${word}s`)}`;
}

export function splitGenres(genres: string | null | undefined): string[] {
  return (genres ?? "")
    .split(",")
    .map((g) => g.trim())
    .filter(Boolean);
}

export function episodeCode(season: number, episode: number): string {
  return `S${season.toString().padStart(2, "0")}E${episode.toString().padStart(2, "0")}`;
}

export function relativeDate(unixSeconds: number | null | undefined): string {
  if (!unixSeconds) return "";
  const diff = Date.now() / 1000 - unixSeconds;
  if (diff < 60) return "Just now";
  if (diff < 3600) return `${Math.floor(diff / 60)} min ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} hr ago`;
  if (diff < 86400 * 7) return `${Math.floor(diff / 86400)} days ago`;
  return new Date(unixSeconds * 1000).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });
}

export function resolution(width: number | null, height: number | null): string | null {
  if (!width || !height) return null;
  if (width >= 3800 || height >= 2100) return "4K";
  if (width >= 1900 || height >= 1060) return "1080p";
  if (width >= 1260 || height >= 700) return "720p";
  return "SD";
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = bytes / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 10 ? 0 : 1)} ${units[i]}`;
}

/** Stable hue from a string, for artwork placeholders. */
export function hueFrom(text: string): number {
  let h = 0;
  for (let i = 0; i < text.length; i++) h = (h * 31 + text.charCodeAt(i)) >>> 0;
  return h % 360;
}

export function shuffleArray<T>(items: T[]): T[] {
  const a = items.slice();
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

export function isTypingTarget(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el) return false;
  const tag = el.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || el.isContentEditable;
}
