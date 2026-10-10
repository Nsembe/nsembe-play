import { useState } from "react";
import { useParams } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { Mic2 } from "lucide-react";
import { Artwork } from "@/components/Artwork";
import { EmptyState } from "@/components/EmptyState";
import { FavoriteButton } from "@/components/MediaActions";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/misc";
import { AlbumCard } from "@/features/library/cards";
import { CollectionActions, CollectionHeader } from "@/features/music/CollectionHeader";
import { TrackList } from "@/features/music/TrackList";
import { usePlayback } from "@/features/player/usePlayback";
import { api } from "@/lib/api";
import { plural } from "@/lib/utils";

export function ArtistDetailPage() {
  const id = Number(useParams().id);
  const { playTracks } = usePlayback();
  const [showAll, setShowAll] = useState(false);
  const { data, isLoading, error } = useQuery({ queryKey: ["artist", id], queryFn: () => api.artist(id) });

  if (isLoading) return <Skeleton className="m-8 mt-24 h-72" />;
  if (error || !data) return <EmptyState className="pt-32" icon={<Mic2 />} title="Artist not found" />;

  const { artist, albums, topTracks } = data;
  const visible = showAll ? topTracks : topTracks.slice(0, 10);

  return (
    <div>
      <CollectionHeader
        seed={artist.name}
        backdrop={artist.artwork}
        eyebrow="Artist"
        title={artist.name}
        cover={<Artwork src={artist.artwork} title={artist.name} kind="artist" eager className="size-[clamp(150px,16vw,220px)] rounded-full" />}
        meta={
          <span>
            {plural(artist.albumCount, "album")} · {plural(artist.trackCount, "song")}
          </span>
        }
      />
      <CollectionActions
        onPlay={() => void playTracks(topTracks, 0)}
        onShuffle={() => void playTracks(topTracks, 0, { shuffle: true })}
        disabled={!topTracks.length}
      >
        <FavoriteButton type="artist" id={artist.id} value={artist.isFavorite} size="lg" />
      </CollectionActions>

      {topTracks.length > 0 && (
        <section className="px-6">
          <h2 className="mb-2 px-2 font-display text-[21px] font-bold tracking-tight">Songs</h2>
          <TrackList tracks={visible} showAlbum onPlay={(i) => void playTracks(topTracks, i)} />
          {topTracks.length > 10 && (
            <Button variant="ghost" size="sm" className="mt-2 ml-2" onClick={() => setShowAll(!showAll)}>
              {showAll ? "Show less" : `Show all ${topTracks.length}`}
            </Button>
          )}
        </section>
      )}

      {albums.length > 0 && (
        <section className="mt-10 px-6">
          <h2 className="mb-2 px-2 font-display text-[21px] font-bold tracking-tight">Albums</h2>
          <div className="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-2">
            {albums.map((a) => (
              <AlbumCard key={a.id} album={a} />
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
