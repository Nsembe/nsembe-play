import { useNavigate, useSearchParams } from "react-router";
import { useInfiniteQuery } from "@tanstack/react-query";
import { History, Play, Sparkles, Trash2 } from "lucide-react";
import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Artwork } from "@/components/Artwork";
import { EmptyState, PageHeader, Segmented } from "@/components/EmptyState";
import { VirtualGrid } from "@/components/VirtualGrid";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/misc";
import { RecentCard, playableToCardProps } from "@/features/library/cards";
import { ConfirmDialog } from "@/features/playlists/PlaylistDialogs";
import { usePlayback } from "@/features/player/usePlayback";
import { api } from "@/lib/api";
import { cn, formatTime, progressFraction, relativeDate } from "@/lib/utils";

export function RecentlyAddedPage() {
  return (
    <div>
      <PageHeader title="Recently Added" subtitle="The newest albums, movies and shows in your library." />
      <div className="px-6">
        <VirtualGrid
          queryKey={["recently-added"]}
          fetchPage={(offset, limit) => api.recentlyAdded({ offset, limit })}
          itemAspect={1}
          extraHeight={76}
          minItemWidth={170}
          gap={8}
          renderItem={(item) => <RecentCard item={item} />}
          empty={<EmptyState icon={<Sparkles />} title="Nothing added yet" description="New media appears here as soon as it is indexed." />}
        />
      </div>
    </div>
  );
}

type Kind = "all" | "music" | "video";

export function RecentlyPlayedPage() {
  const navigate = useNavigate();
  const qc = useQueryClient();
  const [params, setParams] = useSearchParams();
  const kind = (params.get("kind") as Kind) ?? "all";
  const [confirm, setConfirm] = useState(false);
  const { playItem } = usePlayback();
  const query = useInfiniteQuery({
    queryKey: ["recently-played", kind],
    queryFn: ({ pageParam }) => api.recentlyPlayed(kind === "all" ? undefined : kind, { offset: pageParam, limit: 50 }),
    initialPageParam: 0,
    getNextPageParam: (last) => (last.offset + last.items.length < last.total ? last.offset + last.items.length : undefined),
  });
  const items = query.data?.pages.flatMap((p) => p.items) ?? [];
  const tracks = items.filter((i) => i.kind === "track");

  return (
    <div>
      <PageHeader
        title="Recently Played"
        actions={
          items.length > 0 && (
            <Button variant="ghost" size="sm" onClick={() => setConfirm(true)}>
              <Trash2 />
              Clear history
            </Button>
          )
        }
      >
        <Segmented<Kind>
          className="mt-6"
          value={kind}
          onChange={(v) => setParams(v === "all" ? {} : { kind: v }, { replace: true })}
          options={[
            { value: "all", label: "All" },
            { value: "music", label: "Music" },
            { value: "video", label: "Movies & TV" },
          ]}
        />
      </PageHeader>
      <div className="px-6">
        {query.isLoading ? (
          <Skeleton className="mx-2 h-72" />
        ) : items.length === 0 ? (
          <EmptyState icon={<History />} title="Nothing played yet" description="Songs, movies and episodes you play will show up here." />
        ) : (
          <div className="flex flex-col">
            {items.map((item) => {
              const props = playableToCardProps(item);
              const progress = item.kind !== "track" && !item.completed ? progressFraction(item.position, item.duration) : 0;
              return (
                <div
                  key={item.mediaId}
                  onDoubleClick={() => void playItem(item, item.kind === "track" ? tracks : undefined)}
                  className="group flex items-center gap-4 rounded-lg px-3 py-2 hover:bg-white/[0.05]"
                >
                  <div className={cn("relative shrink-0 overflow-hidden rounded", item.kind === "track" ? "size-12" : "h-12 w-[85px]")}>
                    <Artwork src={item.kind === "movie" ? (item.backdrop ?? item.artwork) : item.artwork} title={item.albumTitle ?? item.title} kind={item.kind === "track" ? "album" : "movie"} className="size-full" />
                    <button
                      type="button"
                      onClick={() => void playItem(item, item.kind === "track" ? tracks : undefined)}
                      className="absolute inset-0 grid place-items-center bg-black/50 opacity-0 transition group-hover:opacity-100"
                      aria-label={`Play ${item.title}`}
                    >
                      <Play className="size-5 fill-current" />
                    </button>
                    {progress > 0 && (
                      <div className="absolute bottom-0 left-0 h-[3px] w-full bg-white/20">
                        <div className="h-full bg-accent" style={{ width: `${progress * 100}%` }} />
                      </div>
                    )}
                  </div>
                  <div className="min-w-0 flex-1">
                    <button type="button" onClick={() => props.to && navigate(props.to)} className="block max-w-full truncate text-left text-[14px] font-medium hover:underline">
                      {item.title}
                    </button>
                    <div className="truncate text-[12.5px] text-muted">{props.subtitle}</div>
                  </div>
                  <div className="hidden w-28 text-right text-[12.5px] text-muted sm:block">{relativeDate(item.lastPlayedAt)}</div>
                  <div className="w-14 text-right text-[13px] text-muted tabular-nums">{formatTime(item.duration)}</div>
                </div>
              );
            })}
            {query.hasNextPage && (
              <Button variant="ghost" className="mx-auto mt-4" onClick={() => void query.fetchNextPage()} disabled={query.isFetchingNextPage}>
                Load more
              </Button>
            )}
          </div>
        )}
      </div>
      <ConfirmDialog
        open={confirm}
        onOpenChange={setConfirm}
        title="Clear play history?"
        description="This clears Recently Played, resume positions and Continue Watching. Your library is not affected."
        confirmLabel="Clear history"
        onConfirm={async () => {
          await api.clearHistory();
          void qc.invalidateQueries();
        }}
      />
    </div>
  );
}
