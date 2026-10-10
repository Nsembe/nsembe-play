import { Fragment } from "react";
import { useParams } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { Clapperboard, Play, RotateCcw } from "lucide-react";
import { EmptyState } from "@/components/EmptyState";
import { AddToPlaylistButton, FavoriteButton, MoreMenu } from "@/components/MediaActions";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/misc";
import { VideoMenuItems, MovieCard } from "@/features/library/cards";
import { DetailHero, MetaDot } from "@/features/movies/DetailHero";
import { usePlayback } from "@/features/player/usePlayback";
import { api } from "@/lib/api";
import { formatBytes, formatDuration, formatTime, progressFraction, remainingLabel, resolution, splitGenres } from "@/lib/utils";

function langName(code: string | null): string | null {
  if (!code) return null;
  try {
    return new Intl.DisplayNames(undefined, { type: "language" }).of(code.length === 3 ? code.slice(0, 2) : code) ?? code;
  } catch {
    return code;
  }
}

export function MovieDetailPage() {
  const id = Number(useParams().id);
  const { playVideo } = usePlayback();
  const { data, isLoading, error } = useQuery({ queryKey: ["movie", id], queryFn: () => api.movie(id) });

  if (isLoading) return <Skeleton className="m-8 mt-24 h-[420px]" />;
  if (error || !data) {
    return <EmptyState className="pt-32" icon={<Clapperboard />} title="Movie not found" description="It may have been removed from your library." />;
  }

  const m = data.movie;
  const res = resolution(m.width, m.height);
  const canResume = !m.completed && (m.position ?? 0) > 5;
  const progress = progressFraction(m.position, m.duration);
  const meta = [m.year?.toString(), formatDuration(m.duration), res].filter(Boolean) as string[];

  const tech = [
    { label: "File", value: `${m.container?.toUpperCase() ?? "Video"} · ${formatBytes(data.fileSize)}` },
    { label: "Video", value: [m.videoCodec?.toUpperCase(), m.width && m.height ? `${m.width}×${m.height}` : null].filter(Boolean).join(" · ") },
    {
      label: "Audio",
      value: data.audioStreams
        .map((a) => [langName(a.language), a.codec?.toUpperCase(), a.channels ? `${a.channels}ch` : null].filter(Boolean).join(" "))
        .join(", "),
    },
    { label: "Subtitles", value: data.subtitleStreams.map((s) => langName(s.language) ?? s.title ?? "Unknown").join(", ") },
  ].filter((t) => t.value);

  return (
    <div>
      <DetailHero
        backdrop={m.backdrop}
        poster={m.posterFull ?? m.poster}
        title={m.title}
        kind="movie"
        overview={m.overview}
        tags={splitGenres(m.genres)}
        meta={meta.map((x, i) => (
          <Fragment key={x}>
            {i > 0 && <MetaDot />}
            <span>{x}</span>
          </Fragment>
        ))}
        actions={
          <>
            <Button variant="primary" size="lg" onClick={() => void playVideo(m.mediaId)}>
              <Play className="fill-current" />
              {canResume ? `Resume from ${formatTime(m.position)}` : "Play"}
            </Button>
            {canResume && (
              <Button variant="secondary" size="lg" onClick={() => void playVideo(m.mediaId, 0)}>
                <RotateCcw />
                Start over
              </Button>
            )}
            <div className="flex items-center gap-1 rounded-full bg-white/[0.08] px-1.5 py-1">
              <FavoriteButton type="movie" id={m.id} value={m.isFavorite} />
              <AddToPlaylistButton mediaIds={[m.mediaId]} />
              <MoreMenu>
                <VideoMenuItems mediaId={m.mediaId} completed={m.completed} />
              </MoreMenu>
            </div>
          </>
        }
      />

      {canResume && (
        <div className="-mt-6 mb-8 flex max-w-md items-center gap-3 px-8 text-[13px] text-muted">
          <div className="h-1 flex-1 overflow-hidden rounded-full bg-white/15">
            <div className="h-full bg-accent" style={{ width: `${progress * 100}%` }} />
          </div>
          {remainingLabel(m.position, m.duration)}
        </div>
      )}

      {tech.length > 0 && (
        <section className="mb-10 px-8">
          <h2 className="mb-3 font-display text-lg font-bold">Details</h2>
          <dl className="grid max-w-3xl grid-cols-[110px_1fr] gap-x-6 gap-y-2 text-[14px]">
            {tech.map((t) => (
              <Fragment key={t.label}>
                <dt className="text-muted">{t.label}</dt>
                <dd className="text-fg/90">{t.value}</dd>
              </Fragment>
            ))}
          </dl>
        </section>
      )}

      {data.moreLikeThis.length > 0 && (
        <section className="px-8">
          <h2 className="mb-4 font-display text-[21px] font-bold tracking-tight">More Like This</h2>
          <div className="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-5">
            {data.moreLikeThis.map((mv) => (
              <MovieCard key={mv.id} movie={mv} />
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
