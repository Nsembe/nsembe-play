import { useCallback } from "react";
import { Link, useNavigate, useSearchParams } from "react-router";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { FolderPlus, Heart, History, ListMusic, Music2, Shuffle, Tags } from "lucide-react";
import { Artwork } from "@/components/Artwork";
import { EmptyState, PageHeader, Segmented } from "@/components/EmptyState";
import { VirtualGrid } from "@/components/VirtualGrid";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/misc";
import { AlbumCard, ArtistCard } from "@/features/library/cards";
import { TrackList } from "@/features/music/TrackList";
import { usePlayback } from "@/features/player/usePlayback";
import { api } from "@/lib/api";
import { hueFrom, plural } from "@/lib/utils";

type Tab = "songs" | "artists" | "albums" | "genres";
type SongSort = "title" | "artist" | "album" | "added";

function NoMusic() {
  const navigate = useNavigate();
  return (
    <EmptyState
      icon={<Music2 />}
      title="No music yet"
      description="Add a music folder to start building your library."
      action={
        <Button variant="primary" onClick={() => navigate("/settings")}>
          <FolderPlus />
          Add music folder
        </Button>
      }
    />
  );
}

function Songs({ sort }: { sort: SongSort }) {
  const { playTracks } = usePlayback();
  const query = useInfiniteQuery({
    queryKey: ["tracks", sort],
    queryFn: ({ pageParam }) => api.tracks({ offset: pageParam, limit: 200, sort }),
    initialPageParam: 0,
    getNextPageParam: (last) => (last.offset + last.items.length < last.total ? last.offset + last.items.length : undefined),
  });
  const tracks = query.data?.pages.flatMap((p) => p.items) ?? [];
  const total = query.data?.pages[0]?.total ?? 0;
  const { hasNextPage, isFetchingNextPage, fetchNextPage } = query;
  const onEnd = useCallback(() => {
    if (hasNextPage && !isFetchingNextPage) void fetchNextPage();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

  if (query.isLoading) return <Skeleton className="h-96" />;
  if (total === 0) return <NoMusic />;
  return <TrackList tracks={tracks} total={total} virtual onEndReached={onEnd} onPlay={(i) => void playTracks(tracks, i)} />;
}

function Genres() {
  const { data, isLoading } = useQuery({ queryKey: ["genres"], queryFn: api.genres });
  if (isLoading) return <Skeleton className="h-64" />;
  if (!data?.length) {
    return <EmptyState icon={<Tags />} title="No genres yet" description="Genres come from the tags in your music files." />;
  }
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-4">
      {data.map((g) => {
        const hue = hueFrom(g.name);
        return (
          <Link
            key={g.name}
            to={`/music/genres/${encodeURIComponent(g.name)}`}
            className="group relative h-28 overflow-hidden rounded-xl p-4 transition hover:brightness-110"
            style={{ background: `linear-gradient(135deg, hsl(${hue} 45% 32%), hsl(${(hue + 50) % 360} 40% 16%))` }}
          >
            <div className="relative z-10 font-display text-lg font-bold">{g.name}</div>
            <div className="relative z-10 text-[12.5px] text-white/70">{plural(g.trackCount, "song")}</div>
            <Artwork
              src={g.artwork}
              title={g.name}
              kind="album"
              className="absolute -right-3 -bottom-3 size-20 rotate-[18deg] rounded-md shadow-card transition group-hover:rotate-[12deg]"
            />
          </Link>
        );
      })}
    </div>
  );
}

export function MusicPage() {
  const [params, setParams] = useSearchParams();
  const tab = (params.get("tab") as Tab) ?? "songs";
  const sort = (params.get("sort") as SongSort) ?? "title";
  const { shuffleAll } = usePlayback();
  const { data: count } = useQuery({ queryKey: ["tracks", "count"], queryFn: () => api.tracks({ limit: 1 }) });

  const shortcuts = [
    { to: "/playlists", label: "Playlists", icon: ListMusic },
    { to: "/recent/played?kind=music", label: "Recently Played", icon: History },
    { to: "/favorites", label: "Favorites", icon: Heart },
  ];

  return (
    <div>
      <PageHeader
        title="Music"
        subtitle={count ? plural(count.total, "song") : undefined}
        actions={
          <>
            {shortcuts.map((s) => (
              <Button key={s.to} variant="ghost" size="sm" asChild>
                <Link to={s.to}>
                  <s.icon />
                  {s.label}
                </Link>
              </Button>
            ))}
            <Button variant="primary" onClick={() => void shuffleAll()} disabled={!count?.total}>
              <Shuffle />
              Shuffle all
            </Button>
          </>
        }
      >
        <div className="mt-6 flex flex-wrap items-center justify-between gap-3">
          <Segmented<Tab>
            value={tab}
            onChange={(v) => setParams(v === "songs" ? {} : { tab: v }, { replace: true })}
            options={[
              { value: "songs", label: "Songs" },
              { value: "artists", label: "Artists" },
              { value: "albums", label: "Albums" },
              { value: "genres", label: "Genres" },
            ]}
          />
          {tab === "songs" && (
            <select
              value={sort}
              onChange={(e) => setParams(e.target.value === "title" ? {} : { sort: e.target.value }, { replace: true })}
              className="h-8 rounded-full bg-white/[0.06] px-3 text-[13px] text-fg outline-none"
              aria-label="Sort songs"
            >
              <option value="title">Title</option>
              <option value="artist">Artist</option>
              <option value="album">Album</option>
              <option value="added">Recently added</option>
            </select>
          )}
        </div>
      </PageHeader>
      <div className="px-8">
        {tab === "songs" && <Songs sort={sort} />}
        {tab === "artists" && (
          <VirtualGrid
            queryKey={["artists"]}
            fetchPage={(offset, limit) => api.artists({ offset, limit })}
            itemAspect={1}
            extraHeight={76}
            minItemWidth={170}
            gap={8}
            renderItem={(a) => <ArtistCard artist={a} />}
            empty={<NoMusic />}
          />
        )}
        {tab === "albums" && (
          <VirtualGrid
            queryKey={["albums", "title"]}
            fetchPage={(offset, limit) => api.albums({ offset, limit, sort: "title" })}
            itemAspect={1}
            extraHeight={76}
            minItemWidth={170}
            gap={8}
            renderItem={(a) => <AlbumCard album={a} />}
            empty={<NoMusic />}
          />
        )}
        {tab === "genres" && <Genres />}
      </div>
    </div>
  );
}
