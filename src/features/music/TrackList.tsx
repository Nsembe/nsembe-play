import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { useNavigate } from "react-router";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Clock3, Disc3, FolderOpen, ListEnd, ListStart, Mic2, Play } from "lucide-react";
import { Artwork } from "@/components/Artwork";
import { AddToPlaylistButton, FavoriteButton, MoreMenu } from "@/components/MediaActions";
import { DropdownMenuItem, DropdownMenuSeparator } from "@/components/ui/dropdown-menu";
import { usePlayback } from "@/features/player/usePlayback";
import { useScrollElement } from "@/layouts/scrollContext";
import { api } from "@/lib/api";
import { cn, formatTime } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";
import type { Track } from "@/types";

const ROW_HEIGHT = 56;

interface TrackListProps {
  tracks: Track[];
  onPlay: (index: number) => void;
  /** "index" numbers rows 1..n, "track" uses the album track number. */
  numbering?: "index" | "track";
  showAlbum?: boolean;
  showArtwork?: boolean;
  /** Virtualize against the main scroll container (large lists). */
  virtual?: boolean;
  /** Total available (for paged lists); triggers onEndReached near the end. */
  total?: number;
  onEndReached?: () => void;
  extraMenu?: (track: Track, index: number) => ReactNode;
}

export function PlayingBars() {
  return (
    <span className="flex h-3.5 items-end gap-[2px]" aria-label="Now playing">
      {[0, 1, 2].map((i) => (
        <span
          key={i}
          className="w-[3px] rounded-sm bg-accent"
          style={{ animation: `eq 900ms ${i * 160}ms ease-in-out infinite alternate`, height: "40%" }}
        />
      ))}
      <style>{`@keyframes eq { from { height: 25% } to { height: 100% } }`}</style>
    </span>
  );
}

function TrackRow({
  track,
  index,
  numbering,
  showAlbum,
  showArtwork,
  onPlay,
  extraMenu,
}: {
  track: Track;
  index: number;
  numbering: "index" | "track";
  showAlbum: boolean;
  showArtwork: boolean;
  onPlay: (index: number) => void;
  extraMenu?: (track: Track, index: number) => ReactNode;
}) {
  const navigate = useNavigate();
  const { enqueue } = usePlayback();
  const isCurrent = usePlayerStore((s) => s.queue[s.index]?.mediaId === track.mediaId);
  const isPlaying = usePlayerStore((s) => !s.paused) && isCurrent;
  const number = numbering === "track" ? (track.trackNumber ?? index + 1) : index + 1;

  return (
    <div
      onDoubleClick={() => onPlay(index)}
      className={cn(
        "group/row grid h-14 items-center gap-4 rounded-lg px-4 transition-colors hover:bg-white/[0.05]",
        showAlbum ? "grid-cols-[2rem_minmax(0,4fr)_minmax(0,3fr)_4.5rem_6.5rem]" : "grid-cols-[2rem_minmax(0,1fr)_4.5rem_6.5rem]",
      )}
    >
      <div className="relative flex justify-end text-sm text-muted tabular-nums">
        <span className={cn("group-hover/row:invisible", isCurrent && "text-accent")}>
          {isPlaying ? <PlayingBars /> : number}
        </span>
        <button
          type="button"
          aria-label={`Play ${track.title}`}
          onClick={() => onPlay(index)}
          className="invisible absolute inset-y-0 right-0 grid place-items-center text-fg group-hover/row:visible"
        >
          <Play className="size-4 fill-current" />
        </button>
      </div>
      <div className="flex min-w-0 items-center gap-3">
        {showArtwork && <Artwork src={track.artwork} title={track.albumTitle ?? track.title} kind="album" className="size-10 shrink-0 rounded" />}
        <div className="min-w-0">
          <div className={cn("truncate text-[14px] font-medium", isCurrent ? "text-accent" : "text-fg")}>{track.title}</div>
          {track.artistName && (
            <button
              type="button"
              onClick={() => track.artistId && navigate(`/music/artists/${track.artistId}`)}
              className="block max-w-full truncate text-left text-[12.5px] text-muted hover:text-fg hover:underline"
            >
              {track.artistName}
            </button>
          )}
        </div>
      </div>
      {showAlbum && (
        <button
          type="button"
          onClick={() => track.albumId && navigate(`/albums/${track.albumId}`)}
          className="truncate text-left text-[13px] text-muted hover:text-fg hover:underline"
        >
          {track.albumTitle}
        </button>
      )}
      <div className="text-right text-[13px] text-muted tabular-nums">{formatTime(track.duration)}</div>
      <div className="flex items-center justify-end gap-0.5">
        <div className={cn(!track.isFavorite && "opacity-0 group-hover/row:opacity-100")}>
          <FavoriteButton type="track" id={track.id} value={track.isFavorite} size="sm" />
        </div>
        <div className="opacity-0 group-hover/row:opacity-100">
          <AddToPlaylistButton mediaIds={[track.mediaId]} />
        </div>
        <div className="opacity-0 group-hover/row:opacity-100 has-[[data-state=open]]:opacity-100">
          <MoreMenu>
            <DropdownMenuItem onSelect={() => void enqueue([track.mediaId], true)}>
              <ListStart />
              Play next
            </DropdownMenuItem>
            <DropdownMenuItem onSelect={() => void enqueue([track.mediaId])}>
              <ListEnd />
              Add to queue
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            {track.albumId && (
              <DropdownMenuItem onSelect={() => navigate(`/albums/${track.albumId}`)}>
                <Disc3 />
                Go to album
              </DropdownMenuItem>
            )}
            {track.artistId && (
              <DropdownMenuItem onSelect={() => navigate(`/music/artists/${track.artistId}`)}>
                <Mic2 />
                Go to artist
              </DropdownMenuItem>
            )}
            <DropdownMenuItem onSelect={() => void api.reveal(track.mediaId)}>
              <FolderOpen />
              Show in folder
            </DropdownMenuItem>
            {extraMenu?.(track, index)}
          </MoreMenu>
        </div>
      </div>
    </div>
  );
}

function Header({ showAlbum }: { showAlbum: boolean }) {
  return (
    <div
      className={cn(
        "mb-1 grid h-9 items-center gap-4 border-b border-line px-4 text-[12px] font-medium tracking-wide text-subtle uppercase",
        showAlbum ? "grid-cols-[2rem_minmax(0,4fr)_minmax(0,3fr)_4.5rem_6.5rem]" : "grid-cols-[2rem_minmax(0,1fr)_4.5rem_6.5rem]",
      )}
    >
      <span className="text-right">#</span>
      <span>Title</span>
      {showAlbum && <span>Album</span>}
      <span className="flex justify-end">
        <Clock3 className="size-4" />
      </span>
      <span />
    </div>
  );
}

export function TrackList({
  tracks,
  onPlay,
  numbering = "index",
  showAlbum = true,
  showArtwork = true,
  virtual = false,
  total,
  onEndReached,
  extraMenu,
}: TrackListProps) {
  const rowProps = { numbering, showAlbum, showArtwork, onPlay, extraMenu };
  if (!virtual) {
    return (
      <div>
        <Header showAlbum={showAlbum} />
        {tracks.map((t, i) => (
          <TrackRow key={`${t.id}-${i}`} track={t} index={i} {...rowProps} />
        ))}
      </div>
    );
  }
  return <VirtualTrackList tracks={tracks} total={total ?? tracks.length} onEndReached={onEndReached} rowProps={rowProps} showAlbum={showAlbum} />;
}

function VirtualTrackList({
  tracks,
  total,
  onEndReached,
  rowProps,
  showAlbum,
}: {
  tracks: Track[];
  total: number;
  onEndReached?: () => void;
  rowProps: Omit<Parameters<typeof TrackRow>[0], "track" | "index">;
  showAlbum: boolean;
}) {
  const scrollRef = useScrollElement();
  const listRef = useRef<HTMLDivElement>(null);
  const [margin, setMargin] = useState(0);

  useLayoutEffect(() => {
    const el = listRef.current;
    const scroller = scrollRef.current;
    if (el && scroller) {
      setMargin(el.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop);
    }
  }, [scrollRef]);

  const virtualizer = useVirtualizer({
    count: total,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 12,
    scrollMargin: margin,
  });
  const rows = virtualizer.getVirtualItems();
  const last = rows[rows.length - 1];
  useEffect(() => {
    if (last && last.index >= tracks.length - 20 && tracks.length < total) onEndReached?.();
  }, [last, tracks.length, total, onEndReached]);

  return (
    <div>
      <Header showAlbum={showAlbum} />
      <div ref={listRef} className="relative" style={{ height: virtualizer.getTotalSize() }}>
        {rows.map((row) => {
          const track = tracks[row.index];
          return (
            <div key={row.key} className="absolute top-0 left-0 w-full" style={{ transform: `translateY(${row.start - margin}px)` }}>
              {track ? <TrackRow track={track} index={row.index} {...rowProps} /> : <div className="h-14" />}
            </div>
          );
        })}
      </div>
    </div>
  );
}
