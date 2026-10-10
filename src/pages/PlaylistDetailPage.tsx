import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { DndContext, PointerSensor, closestCenter, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, arrayMove, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Clapperboard, GripVertical, ListMusic, ListEnd, Pencil, Play, Trash2, Tv, X } from "lucide-react";
import { toast } from "sonner";
import { Artwork, PlaylistCover } from "@/components/Artwork";
import { EmptyState } from "@/components/EmptyState";
import { MoreMenu } from "@/components/MediaActions";
import { DropdownMenuItem, DropdownMenuSeparator } from "@/components/ui/dropdown-menu";
import { Skeleton, Tip } from "@/components/ui/misc";
import { CollectionActions, CollectionHeader } from "@/features/music/CollectionHeader";
import { PlayingBars } from "@/features/music/TrackList";
import { ConfirmDialog, EditPlaylistDialog } from "@/features/playlists/PlaylistDialogs";
import { usePlayback } from "@/features/player/usePlayback";
import { api } from "@/lib/api";
import { cn, formatLongDuration, formatTime, plural, relativeDate } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";
import type { PlaylistEntry } from "@/types";

function EntryRow({ entry, index, onPlay, onRemove }: { entry: PlaylistEntry; index: number; onPlay: () => void; onRemove: () => void }) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: entry.itemId });
  const m = entry.media;
  const isCurrent = usePlayerStore((s) => s.queue[s.index]?.mediaId === m.mediaId);
  const playing = usePlayerStore((s) => !s.paused) && isCurrent;
  const KindIcon = m.kind === "movie" ? Clapperboard : m.kind === "episode" ? Tv : null;
  return (
    <div
      ref={setNodeRef}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      onDoubleClick={onPlay}
      className={cn(
        "group grid h-14 grid-cols-[1.5rem_2rem_minmax(0,1fr)_8rem_4.5rem_2.5rem] items-center gap-3 rounded-lg px-3 hover:bg-white/[0.05]",
        isDragging && "z-10 bg-surface-3 shadow-pop",
      )}
    >
      <button type="button" className="cursor-grab text-subtle opacity-0 group-hover:opacity-100 active:cursor-grabbing" aria-label="Drag to reorder" {...attributes} {...listeners}>
        <GripVertical className="size-4" />
      </button>
      <div className="relative flex justify-end text-sm text-muted tabular-nums">
        <span className={cn("group-hover:invisible", isCurrent && "text-accent")}>{playing ? <PlayingBars /> : index + 1}</span>
        <button type="button" onClick={onPlay} className="invisible absolute inset-y-0 right-0 grid place-items-center text-fg group-hover:visible" aria-label={`Play ${m.title}`}>
          <Play className="size-4 fill-current" />
        </button>
      </div>
      <div className="flex min-w-0 items-center gap-3">
        <Artwork src={m.artwork} title={m.albumTitle ?? m.title} kind={m.kind === "track" ? "album" : "movie"} className={cn("shrink-0 rounded", m.kind === "track" ? "size-10" : "h-10 w-[71px]")} />
        <div className="min-w-0">
          <div className={cn("flex items-center gap-1.5 truncate text-[14px] font-medium", isCurrent && "text-accent")}>
            {KindIcon && <KindIcon className="size-3.5 shrink-0 text-muted" />}
            <span className="truncate">{m.title}</span>
          </div>
          <div className="truncate text-[12.5px] text-muted">{m.subtitle}</div>
        </div>
      </div>
      <div className="truncate text-[12.5px] text-muted">{relativeDate(entry.addedAt)}</div>
      <div className="text-right text-[13px] text-muted tabular-nums">{formatTime(m.duration)}</div>
      <Tip label="Remove from playlist">
        <button type="button" onClick={onRemove} className="grid place-items-center rounded-full p-1.5 text-muted opacity-0 hover:text-fg group-hover:opacity-100">
          <X className="size-4" />
        </button>
      </Tip>
    </div>
  );
}

export function PlaylistDetailPage() {
  const id = Number(useParams().id);
  const navigate = useNavigate();
  const qc = useQueryClient();
  const { playItem, playTracks, enqueue } = usePlayback();
  const { data, isLoading, error } = useQuery({ queryKey: ["playlist", id], queryFn: () => api.playlist(id) });
  const [entries, setEntries] = useState<PlaylistEntry[]>([]);
  const [editing, setEditing] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }));

  useEffect(() => {
    if (data) setEntries(data.entries);
  }, [data]);

  if (isLoading) return <Skeleton className="m-8 mt-24 h-72" />;
  if (error || !data) return <EmptyState className="pt-32" icon={<ListMusic />} title="Playlist not found" />;

  const p = data.playlist;
  const items = entries.map((e) => e.media);
  const tracks = items.filter((i) => i.kind === "track");

  const onDragEnd = async (e: DragEndEvent) => {
    if (!e.over || e.active.id === e.over.id) return;
    const from = entries.findIndex((x) => x.itemId === e.active.id);
    const to = entries.findIndex((x) => x.itemId === e.over!.id);
    const next = arrayMove(entries, from, to);
    setEntries(next);
    try {
      await api.reorderPlaylist(id, next.map((x) => x.itemId));
      void qc.invalidateQueries({ queryKey: ["playlists"] });
    } catch (err) {
      setEntries(entries);
      toast.error("Couldn't reorder playlist", { description: (err as Error).message });
    }
  };

  const remove = async (entry: PlaylistEntry) => {
    setEntries((cur) => cur.filter((x) => x.itemId !== entry.itemId));
    await api.removeFromPlaylist(entry.itemId);
    void qc.invalidateQueries({ queryKey: ["playlist", id] });
    void qc.invalidateQueries({ queryKey: ["playlists"] });
  };

  return (
    <div>
      <CollectionHeader
        seed={p.name}
        backdrop={p.coverArtworks[0]}
        eyebrow="Playlist"
        title={p.name}
        description={p.description}
        cover={<PlaylistCover artworks={p.coverArtworks} title={p.name} className="size-[clamp(160px,18vw,232px)] rounded-lg" />}
        meta={
          <>
            <span>{plural(p.itemCount, "item")}</span>
            {p.duration > 0 && <span className="text-fg/60">· {formatLongDuration(p.duration)}</span>}
          </>
        }
      />
      <CollectionActions
        disabled={!items.length}
        onPlay={() => items[0] && void playItem(items[0], items)}
        onShuffle={tracks.length ? () => void playTracks(tracks, 0, { shuffle: true }) : undefined}
      >
        <MoreMenu>
          <DropdownMenuItem onSelect={() => setEditing(true)}>
            <Pencil />
            Edit details
          </DropdownMenuItem>
          {tracks.length > 0 && (
            <DropdownMenuItem onSelect={() => void enqueue(tracks.map((t) => t.mediaId))}>
              <ListEnd />
              Add songs to queue
            </DropdownMenuItem>
          )}
          <DropdownMenuSeparator />
          <DropdownMenuItem onSelect={() => setDeleting(true)} className="text-danger [&_svg]:!text-danger">
            <Trash2 />
            Delete playlist
          </DropdownMenuItem>
        </MoreMenu>
      </CollectionActions>

      <div className="px-6">
        {entries.length === 0 ? (
          <EmptyState
            icon={<ListMusic />}
            title="This playlist is empty"
            description="Use “Add to playlist” on songs, albums, movies or episodes to fill it."
          />
        ) : (
          <>
            <div className="mb-1 grid h-9 grid-cols-[1.5rem_2rem_minmax(0,1fr)_8rem_4.5rem_2.5rem] items-center gap-3 border-b border-line px-3 text-[12px] font-medium tracking-wide text-subtle uppercase">
              <span />
              <span className="text-right">#</span>
              <span>Title</span>
              <span>Added</span>
              <span className="text-right">Time</span>
              <span />
            </div>
            <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={(e) => void onDragEnd(e)}>
              <SortableContext items={entries.map((e) => e.itemId)} strategy={verticalListSortingStrategy}>
                {entries.map((entry, i) => (
                  <EntryRow key={entry.itemId} entry={entry} index={i} onPlay={() => void playItem(entry.media, items)} onRemove={() => void remove(entry)} />
                ))}
              </SortableContext>
            </DndContext>
          </>
        )}
      </div>

      <EditPlaylistDialog playlist={p} open={editing} onOpenChange={setEditing} />
      <ConfirmDialog
        open={deleting}
        onOpenChange={setDeleting}
        title={`Delete “${p.name}”?`}
        description="The playlist will be removed. Your media files are not affected."
        confirmLabel="Delete"
        onConfirm={async () => {
          await api.deletePlaylist(id);
          void qc.invalidateQueries();
          toast.success("Playlist deleted");
          navigate("/playlists");
        }}
      />
    </div>
  );
}
