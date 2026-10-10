// Bridges Rust engine events into client state and the query cache.

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { QueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { useLibraryStore } from "@/stores/libraryStore";
import { usePlayerStore } from "@/stores/playerStore";
import type { Notice, PlayerEnded, PlayerState, ScanProgress } from "@/types";

function debounce(fn: () => void, ms: number) {
  let t: ReturnType<typeof setTimeout> | undefined;
  return () => {
    clearTimeout(t);
    t = setTimeout(fn, ms);
  };
}

export async function startEventBridge(queryClient: QueryClient): Promise<UnlistenFn> {
  const refreshLibrary = debounce(() => void queryClient.invalidateQueries({ predicate: (q) => q.queryKey[0] !== "settings" }), 300);
  const refreshArtwork = debounce(() => void queryClient.invalidateQueries({ predicate: (q) => q.queryKey[0] !== "settings" }), 1500);
  // While indexing, refresh periodically so new items appear progressively.
  const refreshDuringScan = debounce(() => refreshLibrary(), 4000);

  const unlisteners = await Promise.all([
    listen<string>("library://changed", () => refreshLibrary()),
    listen("library://artwork", () => refreshArtwork()),
    listen<ScanProgress>("library://scan-progress", (e) => {
      useLibraryStore.getState().setScan(e.payload);
      if (e.payload.phase === "indexing" && e.payload.processed > 0) refreshDuringScan();
    }),
    listen<PlayerState>("player://state", (e) => {
      const s = e.payload;
      if (s.engine === "audio") usePlayerStore.getState().applyMpvAudio(s);
      else usePlayerStore.getState().applyMpvVideo(s);
      // History/continue-watching changes as playback progresses.
      if (s.engine === "video" && !s.active) refreshLibrary();
    }),
    listen<PlayerEnded>("player://ended", (e) => {
      usePlayerStore.getState().onMpvEnded(e.payload);
      refreshLibrary();
    }),
    listen<Notice>("app://notice", (e) => {
      const n = e.payload;
      const fn = n.level === "error" ? toast.error : n.level === "warning" ? toast.warning : toast.info;
      fn(n.title, { description: n.message });
    }),
  ]);
  return () => unlisteners.forEach((u) => u());
}
