import { useNavigate } from "react-router";
import { ListMusic, Loader2, Maximize2 } from "lucide-react";
import { Artwork } from "@/components/Artwork";
import { FavoriteButton } from "@/components/MediaActions";
import { Tip } from "@/components/ui/misc";
import { cn } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";
import { useUiStore } from "@/stores/uiStore";
import { SeekBar, TransportControls, VolumeControl } from "./controls";

/** Persistent music player docked at the bottom of the window. */
export function PlayerBar() {
  const navigate = useNavigate();
  const current = usePlayerStore((s) => s.queue[s.index] ?? null);
  const buffering = usePlayerStore((s) => s.buffering && !s.paused);
  const queueOpen = useUiStore((s) => s.queueOpen);
  const setQueueOpen = useUiStore((s) => s.setQueueOpen);
  const setNowPlayingOpen = useUiStore((s) => s.setNowPlayingOpen);

  if (!current) return null;

  return (
    <footer className="relative z-40 grid h-[84px] shrink-0 grid-cols-[minmax(0,1fr)_minmax(0,1.6fr)_minmax(0,1fr)] items-center gap-4 border-t border-line bg-bg-deep/95 px-4 backdrop-blur-xl">
      <div className="flex min-w-0 items-center gap-3.5">
        <button
          type="button"
          onClick={() => setNowPlayingOpen(true)}
          className="group relative shrink-0 overflow-hidden rounded-md shadow-card"
          aria-label="Open Now Playing"
        >
          <Artwork src={current.artwork} title={current.albumTitle ?? current.title} kind="album" className="size-14" />
          <span className="absolute inset-0 grid place-items-center bg-black/50 opacity-0 transition group-hover:opacity-100">
            <Maximize2 className="size-4" />
          </span>
        </button>
        <div className="min-w-0">
          <button
            type="button"
            onClick={() => current.albumId && navigate(`/albums/${current.albumId}`)}
            className="block max-w-full truncate text-left text-[14px] font-semibold hover:underline"
          >
            {current.title}
          </button>
          <button
            type="button"
            onClick={() => current.artistId && navigate(`/music/artists/${current.artistId}`)}
            className="block max-w-full truncate text-left text-[12.5px] text-muted hover:text-fg hover:underline"
          >
            {current.subtitle}
          </button>
        </div>
        {current.trackId && <FavoriteButton key={current.trackId} type="track" id={current.trackId} value={current.isFavorite} size="sm" className="shrink-0" />}
        {buffering && <Loader2 className="size-4 shrink-0 animate-spin text-muted" />}
      </div>

      <div className="flex flex-col items-center gap-1.5">
        <TransportControls />
        <SeekBar className="max-w-[620px]" />
      </div>

      <div className="flex items-center justify-end gap-1">
        <Tip label="Queue">
          <button
            type="button"
            onClick={() => setQueueOpen(!queueOpen)}
            className={cn("grid size-8 place-items-center rounded-full text-muted hover:text-fg", queueOpen && "text-accent hover:text-accent")}
          >
            <ListMusic className="size-[18px]" />
          </button>
        </Tip>
        <Tip label="Now Playing">
          <button
            type="button"
            onClick={() => setNowPlayingOpen(true)}
            className="grid size-8 place-items-center rounded-full text-muted hover:text-fg"
          >
            <Maximize2 className="size-4" />
          </button>
        </Tip>
        <VolumeControl className="ml-1" />
      </div>
    </footer>
  );
}
