// Shared actions: favorite toggle, add-to-playlist and "more" menus.

import { useEffect, useState, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Heart, ListPlus, MoreHorizontal, Plus } from "lucide-react";
import { toast } from "sonner";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";
import { useUiStore } from "@/stores/uiStore";
import type { FavoriteType } from "@/types";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Tip } from "@/components/ui/misc";

export function FavoriteButton({
  type,
  id,
  value,
  className,
  size = "md",
}: {
  type: FavoriteType;
  id: number;
  value: boolean;
  className?: string;
  size?: "sm" | "md" | "lg";
}) {
  const qc = useQueryClient();
  const [optimistic, setOptimistic] = useState<boolean | null>(null);
  // Fresh data from the server supersedes the optimistic value.
  useEffect(() => setOptimistic(null), [value]);
  const active = optimistic ?? value;
  const mutation = useMutation({
    mutationFn: () => api.toggleFavorite(type, id),
    onMutate: () => setOptimistic(!active),
    onSuccess: (fav) => {
      setOptimistic(fav);
      void qc.invalidateQueries();
    },
    onError: (e) => {
      setOptimistic(null);
      toast.error("Couldn't update favorites", { description: String((e as Error).message) });
    },
  });
  const iconSize = size === "sm" ? "size-4" : size === "lg" ? "size-6" : "size-[18px]";
  return (
    <Tip label={active ? "Remove from Favorites" : "Add to Favorites"}>
      <button
        type="button"
        aria-pressed={active}
        onClick={(e) => {
          e.stopPropagation();
          e.preventDefault();
          mutation.mutate();
        }}
        className={cn(
          "grid place-items-center rounded-full p-1.5 transition-colors",
          active ? "text-accent" : "text-muted hover:text-fg",
          className,
        )}
      >
        <Heart className={cn(iconSize, active && "fill-current")} />
      </button>
    </Tip>
  );
}

/** Submenu content listing playlists. Use inside a DropdownMenu. */
export function AddToPlaylistSub({ mediaIds, label = "Add to playlist" }: { mediaIds: number[]; label?: string }) {
  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger>
        <ListPlus />
        {label}
      </DropdownMenuSubTrigger>
      <DropdownMenuSubContent>
        <PlaylistItems mediaIds={mediaIds} />
      </DropdownMenuSubContent>
    </DropdownMenuSub>
  );
}

/** Standalone "+" button that opens the playlist picker. */
export function AddToPlaylistButton({ mediaIds, className }: { mediaIds: number[]; className?: string }) {
  return (
    <DropdownMenu>
      <Tip label="Add to playlist">
        <DropdownMenuTrigger asChild>
          <button
            type="button"
            onClick={(e) => e.stopPropagation()}
            className={cn("grid place-items-center rounded-full p-1.5 text-muted transition-colors hover:text-fg", className)}
          >
            <ListPlus className="size-[18px]" />
          </button>
        </DropdownMenuTrigger>
      </Tip>
      <DropdownMenuContent align="end" onClick={(e) => e.stopPropagation()}>
        <DropdownMenuLabel>Add to playlist</DropdownMenuLabel>
        <PlaylistItems mediaIds={mediaIds} />
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function PlaylistItems({ mediaIds }: { mediaIds: number[] }) {
  const qc = useQueryClient();
  const openCreate = useUiStore((s) => s.openCreatePlaylist);
  const { data: playlists } = useQuery({ queryKey: ["playlists"], queryFn: api.playlists });
  return (
    <>
      <DropdownMenuItem onSelect={() => openCreate(mediaIds)}>
        <Plus />
        New playlist…
      </DropdownMenuItem>
      {playlists && playlists.length > 0 && <DropdownMenuSeparator />}
      <div className="max-h-72 overflow-y-auto">
        {playlists?.map((p) => (
          <DropdownMenuItem
            key={p.id}
            onSelect={async () => {
              try {
                const added = await api.addToPlaylist(p.id, mediaIds);
                void qc.invalidateQueries({ queryKey: ["playlists"] });
                void qc.invalidateQueries({ queryKey: ["playlist", p.id] });
                if (added === 0) toast.info(`Already in ${p.name}`);
                else toast.success(`Added to ${p.name}`);
              } catch (e) {
                toast.error("Couldn't add to playlist", { description: (e as Error).message });
              }
            }}
          >
            <span className="truncate">{p.name}</span>
          </DropdownMenuItem>
        ))}
      </div>
    </>
  );
}

/** "…" button with arbitrary menu content. */
export function MoreMenu({ children, className, label = "More" }: { children: ReactNode; className?: string; label?: string }) {
  return (
    <DropdownMenu>
      <Tip label={label}>
        <DropdownMenuTrigger asChild>
          <button
            type="button"
            onClick={(e) => e.stopPropagation()}
            className={cn("grid place-items-center rounded-full p-1.5 text-muted transition-colors hover:text-fg", className)}
          >
            <MoreHorizontal className="size-[18px]" />
          </button>
        </DropdownMenuTrigger>
      </Tip>
      <DropdownMenuContent align="end" onClick={(e) => e.stopPropagation()}>
        {children}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
