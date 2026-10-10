// Domain cards: each knows how to render and play its kind of item.

import { useQueryClient } from "@tanstack/react-query";
import { CheckCircle2, Circle, FolderOpen, RotateCcw } from "lucide-react";
import { toast } from "sonner";
import { AddToPlaylistButton, FavoriteButton, MoreMenu } from "@/components/MediaActions";
import { PosterCard, SquareCard, WideCard } from "@/components/MediaCards";
import { DropdownMenuItem, DropdownMenuSeparator } from "@/components/ui/dropdown-menu";
import { usePlayback } from "@/features/player/usePlayback";
import { api } from "@/lib/api";
import { formatDuration, progressFraction, remainingLabel, watchedLabel } from "@/lib/utils";
import type { Album, Artist, ContinueItem, Movie, PlayableItem, Playlist, RecentItem, Show } from "@/types";

/** Menu entries shared by movies and episodes. */
export function VideoMenuItems({ mediaId, completed }: { mediaId: number; completed: boolean | null }) {
  const qc = useQueryClient();
  const { playVideo } = usePlayback();
  const setWatched = async (watched: boolean) => {
    await api.setWatched(mediaId, watched);
    void qc.invalidateQueries();
    toast.success(watched ? "Marked as watched" : "Marked as unwatched");
  };
  return (
    <>
      <DropdownMenuItem onSelect={() => void playVideo(mediaId, 0)}>
        <RotateCcw />
        Play from beginning
      </DropdownMenuItem>
      {completed ? (
        <DropdownMenuItem onSelect={() => void setWatched(false)}>
          <Circle />
          Mark as unwatched
        </DropdownMenuItem>
      ) : (
        <DropdownMenuItem onSelect={() => void setWatched(true)}>
          <CheckCircle2 />
          Mark as watched
        </DropdownMenuItem>
      )}
      <DropdownMenuSeparator />
      <DropdownMenuItem onSelect={() => void api.reveal(mediaId)}>
        <FolderOpen />
        Show in folder
      </DropdownMenuItem>
    </>
  );
}

export function MovieCard({ movie, className }: { movie: Movie; className?: string }) {
  const { playVideo } = usePlayback();
  const meta = [movie.year, formatDuration(movie.duration)].filter(Boolean).join(" · ");
  return (
    <PosterCard
      className={className}
      title={movie.title}
      subtitle={movie.completed ? `${meta}${meta ? " · " : ""}Watched` : meta}
      artwork={movie.poster ?? movie.backdrop}
      kind="movie"
      to={`/movies/${movie.id}`}
      onPlay={() => void playVideo(movie.mediaId)}
      progress={movie.completed ? 0 : progressFraction(movie.position, movie.duration)}
      actions={
        <>
          <FavoriteButton type="movie" id={movie.id} value={movie.isFavorite} />
          <AddToPlaylistButton mediaIds={[movie.mediaId]} />
          <MoreMenu>
            <VideoMenuItems mediaId={movie.mediaId} completed={movie.completed} />
          </MoreMenu>
        </>
      }
    />
  );
}

export function ShowCard({ show, className }: { show: Show; className?: string }) {
  const { playVideo } = usePlayback();
  const play = async () => {
    const detail = await api.show(show.id);
    if (detail.nextUp) await playVideo(detail.nextUp.mediaId);
  };
  const seasons = show.seasonCount === 1 ? "1 season" : `${show.seasonCount} seasons`;
  return (
    <PosterCard
      className={className}
      title={show.title}
      subtitle={`${seasons} · ${show.episodeCount} episodes`}
      artwork={show.poster ?? show.backdrop}
      kind="show"
      to={`/tv/${show.id}`}
      onPlay={() => void play()}
      progress={show.episodeCount ? show.watchedCount / show.episodeCount : 0}
      actions={<FavoriteButton type="tv_show" id={show.id} value={show.isFavorite} />}
    />
  );
}

export function AlbumCard({ album, className }: { album: Album; className?: string }) {
  const { playTracks } = usePlayback();
  return (
    <SquareCard
      className={className}
      title={album.title}
      subtitle={[album.year, album.artistName].filter(Boolean).join(" · ")}
      artwork={album.artwork}
      kind="album"
      to={`/albums/${album.id}`}
      onPlay={async () => {
        const detail = await api.album(album.id);
        await playTracks(detail.tracks, 0);
      }}
    />
  );
}

export function ArtistCard({ artist, className }: { artist: Artist; className?: string }) {
  const { playTracks } = usePlayback();
  return (
    <SquareCard
      className={className}
      round
      title={artist.name}
      subtitle="Artist"
      artwork={artist.artwork}
      kind="artist"
      to={`/music/artists/${artist.id}`}
      onPlay={async () => {
        const detail = await api.artist(artist.id);
        await playTracks(detail.topTracks, 0);
      }}
    />
  );
}

export function PlaylistCard({ playlist, className }: { playlist: Playlist; className?: string }) {
  const { playItem } = usePlayback();
  return (
    <SquareCard
      className={className}
      title={playlist.name}
      subtitle={playlist.itemCount === 1 ? "1 item" : `${playlist.itemCount} items`}
      artwork={playlist.artwork}
      mosaic={playlist.coverArtworks}
      kind="playlist"
      to={`/playlists/${playlist.id}`}
      onPlay={
        playlist.itemCount > 0
          ? async () => {
              const detail = await api.playlist(playlist.id);
              const items = detail.entries.map((e) => e.media);
              if (items[0]) await playItem(items[0], items);
            }
          : undefined
      }
    />
  );
}

export function RecentCard({ item, className }: { item: RecentItem; className?: string }) {
  const to = item.kind === "album" ? `/albums/${item.id}` : item.kind === "movie" ? `/movies/${item.id}` : `/tv/${item.id}`;
  const label = item.kind === "album" ? "Album" : item.kind === "movie" ? "Movie" : "TV Show";
  return (
    <SquareCard
      className={className}
      title={item.title}
      subtitle={[label, item.subtitle].filter(Boolean).join(" · ")}
      artwork={item.artwork}
      kind={item.kind === "album" ? "album" : item.kind === "movie" ? "movie" : "show"}
      contain={item.kind !== "album"}
      to={to}
    />
  );
}

/** Splits "Show · S01E02" into its parts. */
export function episodeParts(item: PlayableItem): { show: string; code: string } {
  const [show, code] = (item.subtitle ?? "").split(" · ");
  return { show: show ?? "", code: code ?? "" };
}

export function ContinueCard({ item, className }: { item: ContinueItem; className?: string }) {
  const { playVideo } = usePlayback();
  const isEpisode = item.kind === "episode";
  const { show, code } = episodeParts(item);
  const title = isEpisode ? show : item.title;
  const detail =
    item.reason === "next"
      ? `Up next · ${code} · ${item.title}`
      : [isEpisode ? `${code} · ${item.title}` : watchedLabel(item.position), remainingLabel(item.position, item.duration)]
          .filter(Boolean)
          .join(" · ");
  const to = item.movieId ? `/movies/${item.movieId}` : item.showId ? `/tv/${item.showId}` : undefined;
  return (
    <WideCard
      className={className}
      title={title}
      subtitle={detail}
      artwork={isEpisode ? (item.artwork ?? item.backdrop) : (item.backdrop ?? item.artwork)}
      kind={isEpisode ? "episode" : "movie"}
      onPlay={() => void playVideo(item.mediaId, item.reason === "next" ? 0 : undefined)}
      to={to}
      progress={item.reason === "resume" ? progressFraction(item.position, item.duration) : 0}
      badge={
        item.reason === "next" ? (
          <span className="rounded-full bg-accent px-2 py-0.5 text-[11px] font-bold text-forest">NEXT EPISODE</span>
        ) : undefined
      }
    />
  );
}

export function playableToCardProps(item: PlayableItem) {
  if (item.kind === "track") {
    return { to: item.albumId ? `/albums/${item.albumId}` : undefined, subtitle: item.subtitle ?? "" };
  }
  if (item.kind === "movie") return { to: item.movieId ? `/movies/${item.movieId}` : undefined, subtitle: "Movie" };
  const { show, code } = episodeParts(item);
  return { to: item.showId ? `/tv/${item.showId}` : undefined, subtitle: `${show} · ${code}` };
}

