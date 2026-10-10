import { useSearchParams } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { Heart } from "lucide-react";
import { EmptyState, PageHeader, Segmented } from "@/components/EmptyState";
import { Skeleton } from "@/components/ui/misc";
import { AlbumCard, ArtistCard, MovieCard, ShowCard } from "@/features/library/cards";
import { TrackList } from "@/features/music/TrackList";
import { usePlayback } from "@/features/player/usePlayback";
import { api } from "@/lib/api";

type Tab = "all" | "songs" | "albums" | "artists" | "movies" | "shows";

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="mb-10">
      <h2 className="mb-3 px-2 font-display text-[21px] font-bold tracking-tight">{title}</h2>
      {children}
    </section>
  );
}

export function FavoritesPage() {
  const [params, setParams] = useSearchParams();
  const tab = (params.get("tab") as Tab) ?? "all";
  const { playTracks } = usePlayback();
  const { data, isLoading } = useQuery({ queryKey: ["favorites"], queryFn: api.favorites });

  const total = data ? data.tracks.length + data.albums.length + data.artists.length + data.movies.length + data.shows.length : 0;
  const show = (t: Tab) => tab === "all" || tab === t;
  const posterGrid = "grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-5 px-2";
  const squareGrid = "grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-2";

  return (
    <div>
      <PageHeader title="Favorites" subtitle="Everything you've loved, in one place.">
        <Segmented<Tab>
          className="mt-6"
          value={tab}
          onChange={(v) => setParams(v === "all" ? {} : { tab: v }, { replace: true })}
          options={[
            { value: "all", label: "All" },
            { value: "songs", label: `Songs${data ? ` · ${data.tracks.length}` : ""}` },
            { value: "albums", label: `Albums${data ? ` · ${data.albums.length}` : ""}` },
            { value: "artists", label: `Artists${data ? ` · ${data.artists.length}` : ""}` },
            { value: "movies", label: `Movies${data ? ` · ${data.movies.length}` : ""}` },
            { value: "shows", label: `TV Shows${data ? ` · ${data.shows.length}` : ""}` },
          ]}
        />
      </PageHeader>
      <div className="px-6">
        {isLoading || !data ? (
          <Skeleton className="mx-2 h-72" />
        ) : total === 0 ? (
          <EmptyState
            icon={<Heart />}
            title="No favorites yet"
            description="Tap the heart on any song, album, artist, movie or show to keep it here."
          />
        ) : (
          <>
            {show("songs") && data.tracks.length > 0 && (
              <Section title="Songs">
                <TrackList tracks={data.tracks} onPlay={(i) => void playTracks(data.tracks, i)} />
              </Section>
            )}
            {show("albums") && data.albums.length > 0 && (
              <Section title="Albums">
                <div className={squareGrid}>
                  {data.albums.map((a) => (
                    <AlbumCard key={a.id} album={a} />
                  ))}
                </div>
              </Section>
            )}
            {show("artists") && data.artists.length > 0 && (
              <Section title="Artists">
                <div className={squareGrid}>
                  {data.artists.map((a) => (
                    <ArtistCard key={a.id} artist={a} />
                  ))}
                </div>
              </Section>
            )}
            {show("movies") && data.movies.length > 0 && (
              <Section title="Movies">
                <div className={posterGrid}>
                  {data.movies.map((m) => (
                    <MovieCard key={m.id} movie={m} />
                  ))}
                </div>
              </Section>
            )}
            {show("shows") && data.shows.length > 0 && (
              <Section title="TV Shows">
                <div className={posterGrid}>
                  {data.shows.map((s) => (
                    <ShowCard key={s.id} show={s} />
                  ))}
                </div>
              </Section>
            )}
          </>
        )}
      </div>
    </div>
  );
}
