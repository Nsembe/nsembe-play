// One place that decides *how* something is played: music goes to the queue,
// video to mpv's window or the built-in player.

import { useCallback } from "react";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { api, ApiError } from "@/lib/api";
import { usePlayerStore } from "@/stores/playerStore";
import { useUiStore } from "@/stores/uiStore";
import type { PlayableItem, Track } from "@/types";

export function trackToPlayable(t: Track): PlayableItem {
  return {
    mediaId: t.mediaId,
    kind: "track",
    path: "",
    title: t.title,
    subtitle: t.artistName,
    artwork: t.artwork,
    backdrop: null,
    duration: t.duration,
    trackId: t.id,
    albumId: t.albumId,
    albumTitle: t.albumTitle,
    artistId: t.artistId,
    movieId: null,
    episodeId: null,
    showId: null,
    position: null,
    completed: null,
    lastPlayedAt: null,
    missing: false,
    isFavorite: t.isFavorite,
  };
}

function showError(e: unknown, fallback: string) {
  const message = e instanceof ApiError || e instanceof Error ? e.message : fallback;
  const title = e instanceof ApiError && e.code === "missing_file" ? "File not found" : fallback;
  toast.error(title, { description: message });
}

export function usePlayback() {
  const navigate = useNavigate();

  /** Plays a movie or episode. `start` undefined = resume where the viewer left off. */
  const playVideo = useCallback(
    async (mediaId: number, start?: number) => {
      await usePlayerStore.getState().pause();
      const engine = useUiStore.getState().engines.video;
      if (engine === "mpv") {
        try {
          await api.play(mediaId, start);
          navigate(`/watch/${mediaId}`);
        } catch (e) {
          showError(e, "Couldn't start playback");
        }
      } else {
        navigate(`/watch/${mediaId}${start != null ? `?t=${Math.floor(start)}` : ""}`);
      }
    },
    [navigate],
  );

  /** Plays a list of songs starting at `index`. Tracks need full paths, so ids are resolved first. */
  const playTracks = useCallback(async (tracks: Track[] | PlayableItem[], index = 0, opts?: { shuffle?: boolean }) => {
    if (tracks.length === 0) return;
    try {
      const ids = tracks.map((t) => t.mediaId);
      const items = await api.playables(ids);
      const startId = tracks[index]?.mediaId;
      const start = Math.max(items.findIndex((i) => i.mediaId === startId), 0);
      await usePlayerStore.getState().playQueue(items, start, opts);
    } catch (e) {
      showError(e, "Couldn't start playback");
    }
  }, []);

  /** Plays any playable item, dispatching on its kind. */
  const playItem = useCallback(
    async (item: PlayableItem, context?: PlayableItem[]) => {
      if (item.kind === "track") {
        const list = (context ?? [item]).filter((i) => i.kind === "track");
        await playTracks(list, Math.max(list.findIndex((i) => i.mediaId === item.mediaId), 0));
      } else {
        await playVideo(item.mediaId);
      }
    },
    [playTracks, playVideo],
  );

  const enqueue = useCallback(async (mediaIds: number[], next = false) => {
    const items = await api.playables(mediaIds);
    const store = usePlayerStore.getState();
    if (next) store.playNext(items);
    else store.enqueue(items);
    toast.success(next ? "Playing next" : "Added to queue");
  }, []);

  const shuffleAll = useCallback(async () => {
    const items = await api.shuffleMusic(300);
    if (items.length === 0) return toast.info("No music yet", { description: "Add a music folder in Settings." });
    await usePlayerStore.getState().playQueue(items, 0, { shuffle: false });
  }, []);

  return { playVideo, playTracks, playItem, enqueue, shuffleAll };
}
