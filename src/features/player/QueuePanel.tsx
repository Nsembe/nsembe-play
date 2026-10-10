import { DndContext, PointerSensor, closestCenter, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical, X } from "lucide-react";
import { Artwork } from "@/components/Artwork";
import { Button } from "@/components/ui/button";
import { cn, formatTime } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";
import { useUiStore } from "@/stores/uiStore";
import type { PlayableItem } from "@/types";
import { PlayingBars } from "@/features/music/TrackList";

function QueueRow({ item, index, id, current }: { item: PlayableItem; index: number; id: string; current?: boolean }) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id, disabled: current });
  const { jumpTo, removeAt } = usePlayerStore.getState();
  return (
    <div
      ref={setNodeRef}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      onDoubleClick={() => !current && void jumpTo(index)}
      className={cn(
        "group flex items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-white/[0.05]",
        isDragging && "z-10 bg-surface-3 shadow-pop",
      )}
    >
      {!current && (
        <button type="button" className="cursor-grab text-subtle opacity-0 group-hover:opacity-100 active:cursor-grabbing" {...attributes} {...listeners} aria-label="Reorder">
          <GripVertical className="size-4" />
        </button>
      )}
      <Artwork src={item.artwork} title={item.albumTitle ?? item.title} kind="album" className="size-10 shrink-0 rounded" />
      <div className="min-w-0 flex-1">
        <div className={cn("truncate text-[13.5px] font-medium", current && "text-accent")}>{item.title}</div>
        <div className="truncate text-[12px] text-muted">{item.subtitle}</div>
      </div>
      {current ? (
        <PlayingBars />
      ) : (
        <>
          <span className="text-[12px] text-muted tabular-nums group-hover:hidden">{formatTime(item.duration)}</span>
          <button type="button" onClick={() => removeAt(index)} className="hidden rounded-full p-1 text-muted hover:text-fg group-hover:block" aria-label="Remove from queue">
            <X className="size-4" />
          </button>
        </>
      )}
    </div>
  );
}

/** Right-hand queue drawer with drag-and-drop reordering. */
export function QueuePanel() {
  const open = useUiStore((s) => s.queueOpen);
  const setOpen = useUiStore((s) => s.setQueueOpen);
  const queue = usePlayerStore((s) => s.queue);
  const index = usePlayerStore((s) => s.index);
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }));

  if (!open) return null;
  const current = queue[index];
  const upcoming = queue.slice(index + 1);
  const ids = upcoming.map((_, i) => `q-${index + 1 + i}`);

  const onDragEnd = (e: DragEndEvent) => {
    if (!e.over || e.active.id === e.over.id) return;
    const from = Number(String(e.active.id).slice(2));
    const to = Number(String(e.over.id).slice(2));
    usePlayerStore.getState().move(from, to);
  };

  return (
    <aside className="flex w-[360px] shrink-0 animate-fade-in flex-col border-l border-line bg-bg-deep/70">
      <div className="flex h-16 items-center justify-between px-5">
        <h2 className="font-display text-lg font-bold">Queue</h2>
        <button type="button" onClick={() => setOpen(false)} className="rounded-full p-1.5 text-muted hover:bg-white/[0.06] hover:text-fg" aria-label="Close queue">
          <X className="size-[18px]" />
        </button>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-3 pb-6">
        {current ? (
          <>
            <div className="px-2 pt-1 pb-2 text-[12px] font-semibold tracking-wide text-subtle uppercase">Now playing</div>
            <QueueRow item={current} index={index} id="current" current />
            <div className="mt-5 flex items-center justify-between px-2 pb-2">
              <span className="text-[12px] font-semibold tracking-wide text-subtle uppercase">Next up</span>
              {upcoming.length > 0 && (
                <Button variant="link" size="sm" onClick={() => void usePlayerStore.getState().clear()}>
                  Clear queue
                </Button>
              )}
            </div>
            {upcoming.length === 0 ? (
              <p className="px-2 text-[13px] text-muted">Nothing queued. Use “Add to queue” on any song.</p>
            ) : (
              <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
                <SortableContext items={ids} strategy={verticalListSortingStrategy}>
                  {upcoming.map((item, i) => (
                    <QueueRow key={ids[i]} id={ids[i]} item={item} index={index + 1 + i} />
                  ))}
                </SortableContext>
              </DndContext>
            )}
          </>
        ) : (
          <p className="px-2 text-[13px] text-muted">Your queue is empty.</p>
        )}
      </div>
    </aside>
  );
}
