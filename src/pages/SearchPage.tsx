import { Link, useSearchParams } from "react-router";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { Clapperboard, Disc3, ListMusic, Mic2, Music2, SearchX, Tv } from "lucide-react";
import { EmptyState } from "@/components/EmptyState";
import { Shelf } from "@/components/Shelf";
import { Skeleton } from "@/components/ui/misc";
import { AlbumCard, ArtistCard, MovieCard, PlaylistCard, ShowCard } from "@/features/library/cards";
import { TrackList } from "@/features/music/TrackList";
import { usePlayback } from "@/features/player/usePlayback";
import { api } from "@/lib/api";

const browse = [
  { to: "/movies", label: "Movies", icon: Clapperboard, hue: 140 },
  { to: "/tv", label: "TV Shows", icon: Tv, hue: 200 },
  { to: "/music", label: "Songs", icon: Music2, hue: 95 },
  { to: "/albums", label: "Albums", icon: Disc3, hue: 260 },
  { to: "/music?tab=artists", label: "Artists", icon: Mic2, hue: 20 },
  { to: "/playlists", label: "Playlists", icon: ListMusic, hue: 320 },
];

export function SearchPage() {
  const [params] = useSearchParams();
  const q = params.get("q")?.trim() ?? "";
  const { playTracks } = usePlayback();
  const { data, isFetching } = useQuery({
    queryKey: ["search", q],
    queryFn: () => api.search(q, 10),
    enabled: q.length > 0,
    placeholderData: keepPreviousData,
  });

  if (!q) {
    return (
      <div className="px-8 pt-24">
        <h1 className="mb-5 font-display text-[28px] font-extrabold tracking-tight">Browse all</h1>
        <div className="grid grid-cols-[repeat(auto-fill,minmax(210px,1fr))] gap-4">
          {browse.map((b) => (
            <Link
              key={b.to}
              to={b.to}
              className="relative h-32 overflow-hidden rounded-xl p-4 transition hover:brightness-110"
              style={{ background: `linear-gradient(135deg, hsl(${b.hue} 45% 30%), hsl(${(b.hue + 40) % 360} 40% 14%))` }}
            >
              <span className="font-display text-xl font-bold">{b.label}</span>
              <b.icon className="absolute -right-2 -bottom-2 size-20 rotate-12 text-white/20" />
            </Link>
          ))}
        </div>
      </div>
    );
  }

  if (!data) {
    return (
      <div className="space-y-6 px-8 pt-24">
        <Skeleton className="h-8 w-60" />
        <Skeleton className="h-60" />
      </div>
    );
  }

  const nothing =
    !data.tracks.length && !data.albums.length && !data.artists.length && !data.movies.length && !data.shows.length && !data.playlists.length;

  if (nothing && !isFetching) {
    return (
      <div className="pt-20">
        <EmptyState icon={<SearchX />} title={`No results for “${q}”`} description="Check the spelling or try a shorter search." />
      </div>
    );
  }

  return (
    <div className="space-y-10 pt-20">
      {data.tracks.length > 0 && (
        <section className="px-6">
          <h2 className="mb-2 px-2 font-display text-[21px] font-bold tracking-tight">Songs</h2>
          <TrackList tracks={data.tracks} onPlay={(i) => void playTracks(data.tracks, i)} />
        </section>
      )}
      {data.artists.length > 0 && (
        <Shelf title="Artists">
          {data.artists.map((a) => (
            <ArtistCard key={a.id} artist={a} className="w-[180px] shrink-0" />
          ))}
        </Shelf>
      )}
      {data.albums.length > 0 && (
        <Shelf title="Albums">
          {data.albums.map((a) => (
            <AlbumCard key={a.id} album={a} className="w-[180px] shrink-0" />
          ))}
        </Shelf>
      )}
      {data.movies.length > 0 && (
        <Shelf title="Movies">
          {data.movies.map((m) => (
            <MovieCard key={m.id} movie={m} className="w-[160px] shrink-0" />
          ))}
        </Shelf>
      )}
      {data.shows.length > 0 && (
        <Shelf title="TV Shows">
          {data.shows.map((s) => (
            <ShowCard key={s.id} show={s} className="w-[160px] shrink-0" />
          ))}
        </Shelf>
      )}
      {data.playlists.length > 0 && (
        <Shelf title="Playlists">
          {data.playlists.map((p) => (
            <PlaylistCard key={p.id} playlist={p} className="w-[180px] shrink-0" />
          ))}
        </Shelf>
      )}
    </div>
  );
}
