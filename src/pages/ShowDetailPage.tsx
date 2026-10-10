import { Fragment, useState } from "react";
import { useParams } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { CheckCircle2, Play, Tv } from "lucide-react";
import { Artwork } from "@/components/Artwork";
import { EmptyState, Segmented } from "@/components/EmptyState";
import { AddToPlaylistButton, FavoriteButton, MoreMenu } from "@/components/MediaActions";
import { ProgressLine } from "@/components/MediaCards";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/misc";
import { VideoMenuItems } from "@/features/library/cards";
import { DetailHero, MetaDot } from "@/features/movies/DetailHero";
import { usePlayback } from "@/features/player/usePlayback";
import { api } from "@/lib/api";
import { cn, episodeCode, formatDuration, progressFraction, splitGenres } from "@/lib/utils";
import type { Episode } from "@/types";

function EpisodeRow({ ep, onPlay }: { ep: Episode; onPlay: () => void }) {
  const progress = ep.completed ? 0 : progressFraction(ep.position, ep.duration);
  return (
    <div
      onClick={onPlay}
      className="group flex cursor-pointer items-start gap-5 rounded-xl p-3 transition-colors hover:bg-white/[0.04]"
    >
      <div className="relative aspect-video w-[220px] shrink-0 overflow-hidden rounded-lg bg-surface-2 ring-1 ring-white/[0.06]">
        <Artwork src={ep.thumb} title={ep.title} kind="episode" className="size-full" />
        <div className="absolute inset-0 grid place-items-center bg-black/40 opacity-0 transition group-hover:opacity-100">
          <span className="grid size-11 place-items-center rounded-full bg-accent text-forest">
            <Play className="size-5 translate-x-px fill-current" />
          </span>
        </div>
        <ProgressLine value={progress} className="absolute bottom-0 left-0" />
      </div>
      <div className="min-w-0 flex-1 py-1">
        <div className="flex items-center gap-2">
          <span className="text-[13px] font-semibold text-muted tabular-nums">{ep.episodeNumber}</span>
          <h3 className="truncate text-[15px] font-semibold">{ep.title}</h3>
          {ep.completed && <CheckCircle2 className="size-4 shrink-0 text-accent" aria-label="Watched" />}
        </div>
        <div className="mt-0.5 text-[12.5px] text-subtle">
          {episodeCode(ep.seasonNumber, ep.episodeNumber)}
          {ep.duration ? ` · ${formatDuration(ep.duration)}` : ""}
        </div>
        {ep.overview && <p className="mt-2 line-clamp-2 text-[13.5px] leading-relaxed text-muted">{ep.overview}</p>}
      </div>
      <div className="flex shrink-0 items-center gap-0.5 self-center opacity-0 transition group-hover:opacity-100 has-[[data-state=open]]:opacity-100">
        <AddToPlaylistButton mediaIds={[ep.mediaId]} />
        <MoreMenu>
          <VideoMenuItems mediaId={ep.mediaId} completed={ep.completed} />
        </MoreMenu>
      </div>
    </div>
  );
}

export function ShowDetailPage() {
  const id = Number(useParams().id);
  const { playVideo } = usePlayback();
  const { data, isLoading, error } = useQuery({ queryKey: ["show", id], queryFn: () => api.show(id) });
  const [season, setSeason] = useState<number | null>(null);

  if (isLoading) return <Skeleton className="m-8 mt-24 h-[420px]" />;
  if (error || !data) return <EmptyState className="pt-32" icon={<Tv />} title="Show not found" />;

  const { show, seasons, episodes, nextUp } = data;
  const activeSeason = season ?? nextUp?.seasonNumber ?? seasons[0]?.seasonNumber ?? 1;
  const seasonEpisodes = episodes.filter((e) => e.seasonNumber === activeSeason);
  const started = nextUp && ((nextUp.position ?? 0) > 5 || episodes.some((e) => e.completed));
  const meta = [
    show.year?.toString(),
    seasons.length === 1 ? "1 season" : `${seasons.length} seasons`,
    `${show.episodeCount} episodes`,
  ].filter(Boolean) as string[];

  return (
    <div>
      <DetailHero
        backdrop={show.backdrop}
        poster={show.posterFull ?? show.poster}
        title={show.title}
        kind="show"
        eyebrow="TV Show"
        overview={show.overview}
        tags={splitGenres(show.genres)}
        meta={meta.map((x, i) => (
          <Fragment key={x}>
            {i > 0 && <MetaDot />}
            <span>{x}</span>
          </Fragment>
        ))}
        actions={
          <>
            {nextUp && (
              <Button variant="primary" size="lg" onClick={() => void playVideo(nextUp.mediaId)}>
                <Play className="fill-current" />
                {started ? `${(nextUp.position ?? 0) > 5 ? "Resume" : "Play"} ${episodeCode(nextUp.seasonNumber, nextUp.episodeNumber)}` : "Play"}
              </Button>
            )}
            <div className="flex items-center gap-1 rounded-full bg-white/[0.08] px-1.5 py-1">
              <FavoriteButton type="tv_show" id={show.id} value={show.isFavorite} />
              <AddToPlaylistButton mediaIds={episodes.map((e) => e.mediaId)} />
            </div>
            {show.watchedCount > 0 && (
              <span className="text-[13px] text-muted">
                {show.watchedCount} of {show.episodeCount} watched
              </span>
            )}
          </>
        }
      />

      <section className="px-8">
        {seasons.length > 1 && (
          <Segmented<string>
            className="mb-5"
            value={String(activeSeason)}
            onChange={(v) => setSeason(Number(v))}
            options={seasons.map((s) => ({
              value: String(s.seasonNumber),
              label: s.seasonNumber === 0 ? "Specials" : `Season ${s.seasonNumber}`,
            }))}
          />
        )}
        <h2 className={cn("mb-2 font-display text-[21px] font-bold tracking-tight", seasons.length > 1 && "sr-only")}>
          {activeSeason === 0 ? "Specials" : `Season ${activeSeason}`}
        </h2>
        <div className="flex flex-col gap-1">
          {seasonEpisodes.map((ep) => (
            <EpisodeRow key={ep.id} ep={ep} onPlay={() => void playVideo(ep.mediaId)} />
          ))}
        </div>
      </section>
    </div>
  );
}
