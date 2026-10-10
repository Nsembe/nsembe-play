import { useCallback } from "react";
import { useParams } from "react-router";
import { useInfiniteQuery } from "@tanstack/react-query";
import { Tags } from "lucide-react";
import { EmptyState } from "@/components/EmptyState";
import { Skeleton } from "@/components/ui/misc";
import { CollectionActions, CollectionHeader } from "@/features/music/CollectionHeader";
import { TrackList } from "@/features/music/TrackList";
import { usePlayback } from "@/features/player/usePlayback";
import { api } from "@/lib/api";
import { hueFrom, plural } from "@/lib/utils";

export function GenreDetailPage() {
  const genre = decodeURIComponent(useParams().name ?? "");
  const { playTracks } = usePlayback();
  const query = useInfiniteQuery({
    queryKey: ["genre", genre],
    queryFn: ({ pageParam }) => api.genreTracks(genre, { offset: pageParam, limit: 200 }),
    initialPageParam: 0,
    getNextPageParam: (last) => (last.offset + last.items.length < last.total ? last.offset + last.items.length : undefined),
  });
  const tracks = query.data?.pages.flatMap((p) => p.items) ?? [];
  const total = query.data?.pages[0]?.total ?? 0;
  const { hasNextPage, isFetchingNextPage, fetchNextPage } = query;
  const onEnd = useCallback(() => {
    if (hasNextPage && !isFetchingNextPage) void fetchNextPage();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);
  const hue = hueFrom(genre);

  return (
    <div>
      <CollectionHeader
        seed={genre}
        eyebrow="Genre"
        title={genre}
        cover={
          <div
            className="grid size-[180px] place-items-center rounded-lg"
            style={{ background: `linear-gradient(135deg, hsl(${hue} 50% 38%), hsl(${(hue + 50) % 360} 45% 18%))` }}
          >
            <Tags className="size-16 text-white/70" />
          </div>
        }
        meta={<span>{plural(total, "song")}</span>}
      />
      <CollectionActions
        onPlay={() => void playTracks(tracks, 0)}
        onShuffle={() => void playTracks(tracks, 0, { shuffle: true })}
        disabled={!tracks.length}
      />
      <div className="px-6">
        {query.isLoading ? (
          <Skeleton className="h-80" />
        ) : total === 0 ? (
          <EmptyState icon={<Tags />} title="No songs in this genre" />
        ) : (
          <TrackList tracks={tracks} total={total} virtual onEndReached={onEnd} onPlay={(i) => void playTracks(tracks, i)} />
        )}
      </div>
    </div>
  );
}
