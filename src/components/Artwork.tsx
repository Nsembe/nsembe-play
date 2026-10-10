import { useState, type ReactNode } from "react";
import { Clapperboard, Disc3, ListMusic, Mic2, Music2, Tv } from "lucide-react";
import { assetUrl } from "@/lib/media";
import { cn, hueFrom } from "@/lib/utils";

export type ArtworkKind = "movie" | "show" | "album" | "track" | "artist" | "playlist" | "episode";

const icons: Record<ArtworkKind, ReactNode> = {
  movie: <Clapperboard />,
  show: <Tv />,
  episode: <Tv />,
  album: <Disc3 />,
  track: <Music2 />,
  artist: <Mic2 />,
  playlist: <ListMusic />,
};

interface ArtworkProps {
  src: string | null | undefined;
  title: string;
  kind: ArtworkKind;
  className?: string;
  /** Show the title on the placeholder (posters). */
  showTitle?: boolean;
  eager?: boolean;
  /** Fit the whole image over a blurred fill (posters inside square tiles). */
  contain?: boolean;
}

/** Lazy-loaded local artwork with a tasteful generated placeholder. */
export function Artwork({ src, title, kind, className, showTitle, eager, contain }: ArtworkProps) {
  const [failed, setFailed] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const url = assetUrl(src);
  const hue = hueFrom(title);

  return (
    <div className={cn("relative overflow-hidden bg-surface-2", className)}>
      {(!url || failed || !loaded) && (
        <div
          className="absolute inset-0 flex flex-col items-center justify-center gap-3 p-4 text-center"
          style={{
            background: `radial-gradient(120% 90% at 20% 0%, hsl(${hue} 32% 22%) 0%, hsl(${(hue + 40) % 360} 28% 12%) 55%, #0b0f0d 100%)`,
          }}
        >
          <div className="text-white/25 [&_svg]:size-8">{icons[kind]}</div>
          {showTitle && (
            <span className="line-clamp-3 font-display text-sm font-semibold text-balance text-white/70">{title}</span>
          )}
        </div>
      )}
      {url && !failed && contain && loaded && (
        <img src={url} alt="" aria-hidden className="absolute inset-0 size-full scale-110 object-cover opacity-50 blur-2xl" />
      )}
      {url && !failed && (
        <img
          src={url}
          alt=""
          loading={eager ? "eager" : "lazy"}
          decoding="async"
          draggable={false}
          onLoad={() => setLoaded(true)}
          onError={() => setFailed(true)}
          className={cn(
            "absolute inset-0 size-full transition-opacity duration-300",
            contain ? "object-contain p-[6%] drop-shadow-xl" : "object-cover",
            loaded ? "opacity-100" : "opacity-0",
          )}
        />
      )}
    </div>
  );
}

/** 2x2 mosaic for playlists without custom artwork. */
export function PlaylistCover({ artworks, title, className }: { artworks: string[]; title: string; className?: string }) {
  if (artworks.length >= 4) {
    return (
      <div className={cn("grid grid-cols-2 grid-rows-2 overflow-hidden bg-surface-2", className)}>
        {artworks.slice(0, 4).map((a, i) => (
          <Artwork key={i} src={a} title={title} kind="album" className="size-full" />
        ))}
      </div>
    );
  }
  return <Artwork src={artworks[0]} title={title} kind="playlist" className={className} />;
}
