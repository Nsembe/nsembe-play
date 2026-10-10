import { Link, useParams } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { Disc3, ListEnd } from "lucide-react";
import { Artwork } from "@/components/Artwork";
import { EmptyState } from "@/components/EmptyState";
import { AddToPlaylistButton, FavoriteButton, MoreMenu } from "@/components/MediaActions";
import { Shelf } from "@/components/Shelf";
import { DropdownMenuItem } from "@/components/ui/dropdown-menu";
import { Skeleton } from "@/components/ui/misc";
import { AlbumCard } from "@/features/library/cards";
import { CollectionActions, CollectionHeader } from "@/features/music/CollectionHeader";
import { TrackList } from "@/features/music/TrackList";
import { usePlayback } from "@/features/player/usePlayback";
import { api } from "@/lib/api";
import { formatLongDuration, plural } from "@/lib/utils";

export function AlbumDetailPage() {
  const id = Number(useParams().id);
  const { playTracks, enqueue } = usePlayback();
  const { data, isLoading, error } = useQuery({ queryKey: ["album", id], queryFn: () => api.album(id) });

  if (isLoading) return <Skeleton className="m-8 mt-24 h-72" />;
  if (error || !data) return <EmptyState className="pt-32" icon={<Disc3 />} title="Album not found" />;

  const { album, tracks, moreByArtist } = data;
  const discs = [...new Set(tracks.map((t) => t.discNumber ?? 1))];
  const ids = tracks.map((t) => t.mediaId);

  return (
    <div>
      <CollectionHeader
        seed={album.title}
        backdrop={album.artwork}
        eyebrow="Album"
        title={album.title}
        cover={<Artwork src={album.artworkFull ?? album.artwork} title={album.title} kind="album" eager className="size-[clamp(160px,18vw,232px)] rounded-lg" />}
        meta={
          <>
            <Link to={`/music/artists/${album.artistId}`} className="font-semibold text-fg hover:underline">
              {album.artistName}
            </Link>
            {album.year && <span>· {album.year}</span>}
            <span>· {plural(album.trackCount, "song")}</span>
            {album.duration > 0 && <span className="text-fg/60">· {formatLongDuration(album.duration)}</span>}
          </>
        }
      />
      <CollectionActions onPlay={() => void playTracks(tracks, 0)} onShuffle={() => void playTracks(tracks, 0, { shuffle: true })} disabled={!tracks.length}>
        <FavoriteButton type="album" id={album.id} value={album.isFavorite} size="lg" />
        <AddToPlaylistButton mediaIds={ids} />
        <MoreMenu>
          <DropdownMenuItem onSelect={() => void enqueue(ids)}>
            <ListEnd />
            Add to queue
          </DropdownMenuItem>
        </MoreMenu>
      </CollectionActions>

      <div className="px-6">
        {discs.length > 1 ? (
          discs.map((d) => {
            const discTracks = tracks.filter((t) => (t.discNumber ?? 1) === d);
            return (
              <div key={d} className="mb-6">
                <div className="flex items-center gap-2 px-4 pb-2 text-[13px] font-semibold text-muted">
                  <Disc3 className="size-4" /> Disc {d}
                </div>
                <TrackList
                  tracks={discTracks}
                  numbering="track"
                  showAlbum={false}
                  showArtwork={false}
                  onPlay={(i) => void playTracks(tracks, tracks.indexOf(discTracks[i]))}
                />
              </div>
            );
          })
        ) : (
          <TrackList tracks={tracks} numbering="track" showAlbum={false} showArtwork={false} onPlay={(i) => void playTracks(tracks, i)} />
        )}
      </div>

      {moreByArtist.length > 0 && (
        <Shelf className="mt-10" title={`More by ${album.artistName}`} to={`/music/artists/${album.artistId}`}>
          {moreByArtist.map((a) => (
            <AlbumCard key={a.id} album={a} className="w-[188px] shrink-0" />
          ))}
        </Shelf>
      )}
    </div>
  );
}
