import { useNavigate } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { FolderPlus, Info, Play } from "lucide-react";
import { EmptyState } from "@/components/EmptyState";
import { SquareCard } from "@/components/MediaCards";
import { Shelf } from "@/components/Shelf";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/misc";
import { AlbumCard, ContinueCard, MovieCard, PlaylistCard, RecentCard, ShowCard, episodeParts, playableToCardProps } from "@/features/library/cards";
import { usePlayback } from "@/features/player/usePlayback";
import { api } from "@/lib/api";
import { assetUrl } from "@/lib/media";
import { formatDuration, remainingLabel, splitGenres } from "@/lib/utils";
import type { HomeFeed } from "@/types";

function greeting() {
  const h = new Date().getHours();
  return h < 12 ? "Good morning" : h < 18 ? "Good afternoon" : "Good evening";
}

/** Large cinematic banner for the most relevant video. */
function Hero({ feed }: { feed: HomeFeed }) {
  const navigate = useNavigate();
  const { playVideo } = usePlayback();
  const resume = feed.continueWatching[0];
  const movie = feed.movies.find((m) => m.backdrop) ?? feed.movies[0];

  let hero: { title: string; eyebrow: string; meta: string; overview?: string | null; image: string | null; play: () => void; info?: () => void; playLabel: string } | null = null;
  if (resume) {
    const isEpisode = resume.kind === "episode";
    const { show, code } = episodeParts(resume);
    hero = {
      title: isEpisode ? show : resume.title,
      eyebrow: resume.reason === "next" ? "Up next" : "Continue watching",
      meta: isEpisode ? `${code} · ${resume.title}` : remainingLabel(resume.position, resume.duration),
      image: resume.backdrop ?? resume.artwork,
      play: () => void playVideo(resume.mediaId, resume.reason === "next" ? 0 : undefined),
      info: resume.movieId ? () => navigate(`/movies/${resume.movieId}`) : resume.showId ? () => navigate(`/tv/${resume.showId}`) : undefined,
      playLabel: resume.reason === "next" ? "Play" : "Resume",
    };
  } else if (movie) {
    hero = {
      title: movie.title,
      eyebrow: "Recently added",
      meta: [movie.year, formatDuration(movie.duration), splitGenres(movie.genres).slice(0, 2).join(", ")].filter(Boolean).join(" · "),
      overview: movie.overview,
      image: movie.backdrop ?? movie.poster,
      play: () => void playVideo(movie.mediaId),
      info: () => navigate(`/movies/${movie.id}`),
      playLabel: "Play",
    };
  }

  if (!hero) {
    return (
      <div className="px-8 pt-24 pb-4">
        <h1 className="font-display text-[34px] font-extrabold tracking-tight">{greeting()}</h1>
      </div>
    );
  }
  const img = assetUrl(hero.image);
  return (
    <section className="relative mb-6 h-[min(62vh,560px)] min-h-[380px] overflow-hidden">
      {img && <img src={img} alt="" className="absolute inset-0 size-full animate-fade-in object-cover object-[center_25%]" />}
      {!img && <div className="absolute inset-0 bg-gradient-to-br from-forest via-surface to-bg" />}
      <div className="absolute inset-0 bg-gradient-to-r from-bg via-bg/70 to-transparent" />
      <div className="absolute inset-0 bg-gradient-to-t from-bg via-bg/10 to-bg/30" />
      <div className="relative flex h-full max-w-2xl animate-rise flex-col justify-end px-8 pb-12">
        <span className="mb-3 text-[12px] font-bold tracking-[0.2em] text-accent uppercase">{hero.eyebrow}</span>
        <h1 className="font-display text-[clamp(2.2rem,4.5vw,3.6rem)] leading-[1.02] font-extrabold tracking-tight text-balance">{hero.title}</h1>
        {hero.meta && <p className="mt-3 text-[15px] font-medium text-fg/80">{hero.meta}</p>}
        {hero.overview && <p className="mt-3 line-clamp-3 max-w-xl text-[15px] leading-relaxed text-muted">{hero.overview}</p>}
        <div className="mt-6 flex gap-3">
          <Button variant="primary" size="lg" onClick={hero.play}>
            <Play className="fill-current" />
            {hero.playLabel}
          </Button>
          {hero.info && (
            <Button variant="secondary" size="lg" onClick={hero.info}>
              <Info />
              More info
            </Button>
          )}
        </div>
      </div>
    </section>
  );
}

export function HomePage() {
  const navigate = useNavigate();
  const { playItem } = usePlayback();
  const { data: feed, isLoading } = useQuery({ queryKey: ["home"], queryFn: api.home });

  if (isLoading || !feed) {
    return (
      <div className="space-y-10 px-8 pt-24">
        <Skeleton className="h-[340px] w-full rounded-2xl" />
        {[0, 1].map((i) => (
          <div key={i} className="flex gap-4">
            {Array.from({ length: 6 }, (_, j) => (
              <Skeleton key={j} className="aspect-[2/3] w-[160px]" />
            ))}
          </div>
        ))}
      </div>
    );
  }

  const s = feed.stats;
  const empty = s.tracks + s.movies + s.episodes === 0;
  if (empty) {
    return (
      <div className="pt-16">
        <EmptyState
          icon={<FolderPlus />}
          title="Your library is empty"
          description="Add your music, movie and TV folders and Nsembe Play will organise everything automatically. Nothing is uploaded or copied."
          action={
            <Button variant="primary" size="lg" onClick={() => navigate("/settings")}>
              <FolderPlus />
              Add media folders
            </Button>
          }
        />
      </div>
    );
  }

  return (
    <div className="space-y-10 pb-6">
      <Hero feed={feed} />

      {feed.continueWatching.length > 0 && (
        <Shelf title="Continue Watching">
          {feed.continueWatching.map((c) => (
            <ContinueCard key={c.mediaId} item={c} className="w-[300px] shrink-0" />
          ))}
        </Shelf>
      )}

      {feed.recentlyAdded.length > 0 && (
        <Shelf title="Recently Added" to="/recent/added">
          {feed.recentlyAdded.map((r) => (
            <RecentCard key={`${r.kind}-${r.id}`} item={r} className="w-[188px] shrink-0" />
          ))}
        </Shelf>
      )}

      {feed.recentlyPlayed.length > 0 && (
        <Shelf title="Recently Played" to="/recent/played">
          {feed.recentlyPlayed.map((p) => {
            const props = playableToCardProps(p);
            return (
              <SquareCard
                key={p.mediaId}
                className="w-[188px] shrink-0"
                title={p.title}
                subtitle={props.subtitle}
                artwork={p.artwork}
                kind={p.kind === "track" ? "album" : p.kind === "movie" ? "movie" : "episode"}
                contain={p.kind === "movie"}
                to={props.to}
                onPlay={() => void playItem(p, p.kind === "track" ? feed.recentlyPlayed : undefined)}
              />
            );
          })}
        </Shelf>
      )}

      {feed.playlists.length > 0 && (
        <Shelf title="Your Playlists" to="/playlists">
          {feed.playlists.map((p) => (
            <PlaylistCard key={p.id} playlist={p} className="w-[188px] shrink-0" />
          ))}
        </Shelf>
      )}

      {feed.albums.length > 0 && (
        <Shelf title="Music" subtitle="Recently added albums" to="/albums">
          {feed.albums.map((a) => (
            <AlbumCard key={a.id} album={a} className="w-[188px] shrink-0" />
          ))}
        </Shelf>
      )}

      {feed.movies.length > 0 && (
        <Shelf title="Movies" to="/movies">
          {feed.movies.map((m) => (
            <MovieCard key={m.id} movie={m} className="w-[168px] shrink-0" />
          ))}
        </Shelf>
      )}

      {feed.shows.length > 0 && (
        <Shelf title="TV Shows" to="/tv">
          {feed.shows.map((sh) => (
            <ShowCard key={sh.id} show={sh} className="w-[168px] shrink-0" />
          ))}
        </Shelf>
      )}
    </div>
  );
}
