// Music player state: queue, shuffle/repeat and the active audio engine.
// Video playback state (mpv window) is mirrored in `video`.

import { create } from "zustand";
import { persist } from "zustand/middleware";
import { toast } from "sonner";
import { HtmlAudioEngine, MpvAudioEngine, type AudioEngine, type EngineCallbacks } from "@/features/player/engines";
import { shuffleArray } from "@/lib/utils";
import type { EngineName, PlayableItem, PlayerEnded, PlayerState } from "@/types";

export type RepeatMode = "off" | "all" | "one";

interface PlayerStore {
  queue: PlayableItem[];
  index: number;
  /** Queue order before shuffling, restored when shuffle is turned off. */
  unshuffled: PlayableItem[] | null;
  shuffle: boolean;
  repeat: RepeatMode;
  position: number;
  duration: number;
  paused: boolean;
  buffering: boolean;
  volume: number;
  muted: boolean;
  engineName: EngineName;
  video: PlayerState | null;

  current: () => PlayableItem | null;
  setEngine: (name: EngineName) => void;
  playQueue: (items: PlayableItem[], startIndex?: number, opts?: { shuffle?: boolean }) => Promise<void>;
  toggle: () => Promise<void>;
  pause: () => Promise<void>;
  next: (auto?: boolean) => Promise<void>;
  previous: () => Promise<void>;
  seek: (position: number) => Promise<void>;
  setVolume: (volume: number) => void;
  toggleMute: () => void;
  toggleShuffle: () => void;
  cycleRepeat: () => void;
  enqueue: (items: PlayableItem[]) => void;
  playNext: (items: PlayableItem[]) => void;
  removeAt: (index: number) => void;
  move: (from: number, to: number) => void;
  jumpTo: (index: number) => Promise<void>;
  clear: () => Promise<void>;

  applyMpvAudio: (state: PlayerState) => void;
  applyMpvVideo: (state: PlayerState) => void;
  onMpvEnded: (e: PlayerEnded) => void;
}

let engine: AudioEngine | null = null;

function getEngine(): AudioEngine {
  const name = usePlayerStore.getState().engineName;
  if (!engine || engine.name !== name) {
    void engine?.stop();
    engine = name === "mpv" ? new MpvAudioEngine() : new HtmlAudioEngine(htmlCallbacks);
    void engine.setVolume(usePlayerStore.getState().volume);
  }
  return engine;
}

const htmlCallbacks: EngineCallbacks = {
  onTime: (position, duration) =>
    usePlayerStore.setState((s) => ({ position, duration: duration ?? s.duration })),
  onPaused: (paused) => usePlayerStore.setState({ paused }),
  onBuffering: (buffering) => usePlayerStore.setState({ buffering }),
  onEnded: () => void usePlayerStore.getState().next(true),
  onError: (message) => {
    toast.error("Playback failed", { description: message });
    usePlayerStore.setState({ paused: true });
  },
};


async function loadIndex(index: number, start = 0) {
  const { queue } = usePlayerStore.getState();
  const item = queue[index];
  if (!item) return;
  usePlayerStore.setState({
    index,
    position: start,
    duration: item.duration ?? 0,
    paused: false,
    buffering: true,
  });
  try {
    await getEngine().load(item, start);
  } catch (e) {
    usePlayerStore.setState({ paused: true, buffering: false });
    const message = e instanceof Error ? e.message : String(e);
    toast.error(`Couldn't play "${item.title}"`, { description: message });
  }
}

export const usePlayerStore = create<PlayerStore>()(
  persist(
    (set, get) => ({
      queue: [],
      index: 0,
      unshuffled: null,
      shuffle: false,
      repeat: "off",
      position: 0,
      duration: 0,
      paused: true,
      buffering: false,
      volume: 80,
      muted: false,
      engineName: "builtin",
      video: null,

      current: () => get().queue[get().index] ?? null,

      setEngine: (engineName) => {
        if (engineName !== get().engineName) {
          void engine?.stop();
          engine = null;
          set({ engineName, paused: true });
        }
      },

      playQueue: async (items, startIndex = 0, opts) => {
        const tracks = items.filter((i) => i.kind === "track" && !i.missing);
        if (tracks.length === 0) return;
        const first = items[startIndex];
        let queue = tracks;
        let index = Math.max(tracks.indexOf(first), 0);
        const shuffle = opts?.shuffle ?? get().shuffle;
        let unshuffled: PlayableItem[] | null = null;
        if (shuffle) {
          unshuffled = tracks;
          const startItem = opts?.shuffle && startIndex === 0 ? null : tracks[index];
          const rest = shuffleArray(tracks.filter((t) => t !== startItem));
          queue = startItem ? [startItem, ...rest] : rest;
          index = 0;
        }
        set({ queue, unshuffled, shuffle, index });
        await loadIndex(index, 0);
      },

      toggle: async () => {
        const s = get();
        if (!s.current()) return;
        if (s.paused) {
          // After an app restart the engine has nothing loaded yet.
          if (!engine || s.position === 0) {
            await loadIndex(s.index, s.position);
          } else {
            set({ paused: false });
            await getEngine().resume().catch(() => loadIndex(s.index, s.position));
          }
        } else {
          set({ paused: true });
          await getEngine().pause();
        }
      },

      pause: async () => {
        if (!get().paused && engine) {
          set({ paused: true });
          await engine.pause().catch(() => {});
        }
      },

      next: async (auto = false) => {
        const { queue, index, repeat } = get();
        if (queue.length === 0) return;
        if (auto && repeat === "one") return loadIndex(index, 0);
        if (index + 1 < queue.length) return loadIndex(index + 1, 0);
        if (repeat === "all") return loadIndex(0, 0);
        // End of the queue.
        set({ paused: true, position: 0 });
        if (!auto) await getEngine().pause();
      },

      previous: async () => {
        const { index, position } = get();
        if (position > 3 || index === 0) {
          await get().seek(0);
          return;
        }
        await loadIndex(index - 1, 0);
      },

      seek: async (position) => {
        set({ position });
        await getEngine().seek(position);
      },

      setVolume: (volume) => {
        const v = Math.round(Math.min(Math.max(volume, 0), 100));
        set({ volume: v, muted: v === 0 ? get().muted : false });
        void engine?.setVolume(v);
        if (get().muted && v > 0) void engine?.setMuted(false);
      },

      toggleMute: () => {
        const muted = !get().muted;
        set({ muted });
        void getEngine().setMuted(muted);
      },

      toggleShuffle: () => {
        const { shuffle, queue, index, unshuffled } = get();
        const current = queue[index];
        if (!shuffle) {
          const rest = shuffleArray(queue.filter((_, i) => i !== index));
          set({ shuffle: true, unshuffled: queue, queue: current ? [current, ...rest] : rest, index: 0 });
        } else {
          const original = unshuffled ?? queue;
          const newIndex = Math.max(original.findIndex((i) => i.mediaId === current?.mediaId), 0);
          set({ shuffle: false, unshuffled: null, queue: original, index: newIndex });
        }
      },

      cycleRepeat: () => {
        const order: RepeatMode[] = ["off", "all", "one"];
        set({ repeat: order[(order.indexOf(get().repeat) + 1) % order.length] });
      },

      enqueue: (items) => {
        const tracks = items.filter((i) => i.kind === "track");
        if (tracks.length === 0) return;
        const wasEmpty = get().queue.length === 0;
        set((s) => ({ queue: [...s.queue, ...tracks], unshuffled: s.unshuffled ? [...s.unshuffled, ...tracks] : null }));
        if (wasEmpty) void loadIndex(0, 0);
      },

      playNext: (items) => {
        const tracks = items.filter((i) => i.kind === "track");
        if (tracks.length === 0) return;
        if (get().queue.length === 0) return get().enqueue(tracks);
        set((s) => {
          const q = s.queue.slice();
          q.splice(s.index + 1, 0, ...tracks);
          return { queue: q };
        });
      },

      removeAt: (i) => {
        const { queue, index } = get();
        if (i === index) return; // the playing item stays
        const q = queue.slice();
        q.splice(i, 1);
        set({ queue: q, index: i < index ? index - 1 : index });
      },

      move: (from, to) => {
        const { queue, index } = get();
        const q = queue.slice();
        const [item] = q.splice(from, 1);
        q.splice(to, 0, item);
        let newIndex = index;
        if (from === index) newIndex = to;
        else if (from < index && to >= index) newIndex = index - 1;
        else if (from > index && to <= index) newIndex = index + 1;
        set({ queue: q, index: newIndex });
      },

      jumpTo: (i) => loadIndex(i, 0),

      clear: async () => {
        await engine?.stop();
        set({ queue: [], index: 0, unshuffled: null, paused: true, position: 0, duration: 0 });
      },

      applyMpvAudio: (state) => {
        if (get().engineName !== "mpv") return;
        const current = get().current();
        // Ignore stale updates for an item we've already moved away from.
        if (state.item && current && state.item.mediaId !== current.mediaId) return;
        set({
          position: state.position,
          duration: state.duration ?? get().duration,
          paused: state.item ? state.paused : get().paused,
          buffering: state.buffering,
        });
      },

      applyMpvVideo: (state) => set({ video: state }),

      onMpvEnded: (e) => {
        if (e.engine === "audio" && get().engineName === "mpv") {
          const current = get().current();
          if (e.reason === "eof" && current?.mediaId === e.mediaId) void get().next(true);
          // mpv failed or exited unexpectedly: reflect that nothing is playing.
          if (e.reason === "error" || e.reason === "closed") set({ paused: true, buffering: false });
        }
      },
    }),
    {
      name: "nsembe-player",
      partialize: (s) => ({
        queue: s.queue.slice(0, 1000),
        index: s.index,
        shuffle: s.shuffle,
        repeat: s.repeat,
        volume: s.volume,
      }),
      // A restored queue starts paused at the beginning of the current song.
      onRehydrateStorage: () => (state) => {
        const duration = state?.queue[state.index]?.duration ?? 0;
        setTimeout(() => usePlayerStore.setState({ paused: true, position: 0, duration }));
      },
    },
  ),
);
