import type { KeyboardEvent, ReactNode } from "react";
import { useNavigate } from "react-router";
import { Play } from "lucide-react";
import { Artwork, PlaylistCover, type ArtworkKind } from "@/components/Artwork";
import { cn } from "@/lib/utils";

interface CardProps {
  title: string;
  subtitle?: ReactNode;
  artwork: string | null | undefined;
  kind: ArtworkKind;
  to?: string;
  onPlay?: () => void;
  /** Small icon buttons shown on hover (favorite, add to playlist, more). */
  actions?: ReactNode;
  className?: string;
}

function useCardNav(to?: string) {
  const navigate = useNavigate();
  return {
    role: to ? "link" : undefined,
    tabIndex: to ? 0 : undefined,
    onClick: to ? () => navigate(to) : undefined,
    onKeyDown: to
      ? (e: KeyboardEvent) => {
          if (e.key === "Enter") navigate(to);
        }
      : undefined,
  };
}

function PlayFab({ onPlay, className }: { onPlay: () => void; className?: string }) {
  return (
    <button
      type="button"
      aria-label="Play"
      onClick={(e) => {
        e.stopPropagation();
        onPlay();
      }}
      className={cn(
        "grid size-12 place-items-center rounded-full bg-accent text-forest shadow-[0_8px_24px_-6px_rgb(120_247_96/0.55)] transition-transform hover:scale-105 active:scale-95",
        className,
      )}
    >
      <Play className="size-5 translate-x-px fill-current" />
    </button>
  );
}

export function ProgressLine({ value, className }: { value: number; className?: string }) {
  if (value <= 0) return null;
  return (
    <div className={cn("h-1 w-full bg-white/15", className)}>
      <div className="h-full rounded-r-full bg-accent" style={{ width: `${Math.max(value * 100, 3)}%` }} />
    </div>
  );
}

/** 2:3 poster for movies and shows. */
export function PosterCard({ title, subtitle, artwork, kind, to, onPlay, actions, className, progress = 0 }: CardProps & { progress?: number }) {
  const nav = useCardNav(to);
  return (
    <div {...nav} className={cn("group/card cursor-pointer outline-none", className)}>
      <div className="relative aspect-[2/3] overflow-hidden rounded-[10px] shadow-card ring-1 ring-white/[0.06] transition duration-200 group-hover/card:ring-white/20 group-focus-visible/card:ring-accent">
        <Artwork src={artwork} title={title} kind={kind} showTitle className="size-full transition-transform duration-500 group-hover/card:scale-[1.03]" />
        <div className="absolute inset-0 bg-gradient-to-t from-black/85 via-black/20 to-transparent opacity-0 transition-opacity duration-200 group-hover/card:opacity-100" />
        {onPlay && (
          <div className="absolute inset-0 grid place-items-center opacity-0 transition duration-200 group-hover/card:opacity-100">
            <PlayFab onPlay={onPlay} className="scale-90 group-hover/card:scale-100" />
          </div>
        )}
        {actions && (
          <div className="absolute right-1.5 bottom-2 left-1.5 flex justify-end gap-0.5 opacity-0 transition-opacity duration-200 group-hover/card:opacity-100 [&_button]:text-white/80 [&_button:hover]:text-white">
            {actions}
          </div>
        )}
        <ProgressLine value={progress} className="absolute bottom-0 left-0" />
      </div>
      <div className="mt-2.5 px-0.5">
        <div className="truncate text-[14px] font-medium text-fg">{title}</div>
        {subtitle && <div className="mt-0.5 truncate text-[12.5px] text-muted">{subtitle}</div>}
      </div>
    </div>
  );
}

/** Square artwork for albums, playlists and (round) artists. */
export function SquareCard({
  title,
  subtitle,
  artwork,
  kind,
  to,
  onPlay,
  actions,
  className,
  round,
  mosaic,
  contain,
}: CardProps & { round?: boolean; mosaic?: string[]; contain?: boolean }) {
  const nav = useCardNav(to);
  return (
    <div
      {...nav}
      className={cn(
        "group/card cursor-pointer rounded-xl p-2.5 outline-none transition-colors duration-200 hover:bg-white/[0.04] focus-visible:bg-white/[0.05]",
        className,
      )}
    >
      <div
        className={cn(
          "relative aspect-square shadow-card",
          round ? "rounded-full" : "rounded-lg",
        )}
      >
        <div className={cn("size-full overflow-hidden", round ? "rounded-full" : "rounded-lg")}>
          {mosaic ? (
            <PlaylistCover artworks={mosaic} title={title} className="size-full" />
          ) : (
            <Artwork src={artwork} title={title} kind={kind} contain={contain} className="size-full" />
          )}
        </div>
        {onPlay && (
          <PlayFab
            onPlay={onPlay}
            className="absolute right-2 bottom-2 translate-y-2 opacity-0 transition duration-200 group-hover/card:translate-y-0 group-hover/card:opacity-100"
          />
        )}
      </div>
      <div className={cn("mt-3 px-0.5", round && "text-center")}>
        <div className="truncate text-[14px] font-semibold text-fg">{title}</div>
        {subtitle && <div className="mt-0.5 line-clamp-1 text-[12.5px] text-muted">{subtitle}</div>}
      </div>
      {actions && <div className="mt-1 flex gap-0.5 opacity-0 transition-opacity group-hover/card:opacity-100">{actions}</div>}
    </div>
  );
}

/** 16:9 card for continue watching and episodes. */
export function WideCard({
  title,
  subtitle,
  artwork,
  kind,
  to,
  onPlay,
  actions,
  className,
  progress = 0,
  badge,
}: CardProps & { progress?: number; badge?: ReactNode }) {
  const nav = useCardNav(to);
  const clickable = to || onPlay;
  return (
    <div
      {...nav}
      onClick={nav.onClick ?? onPlay}
      className={cn("group/card outline-none", clickable && "cursor-pointer", className)}
    >
      <div className="relative aspect-video overflow-hidden rounded-[10px] shadow-card ring-1 ring-white/[0.06] transition group-hover/card:ring-white/20">
        <Artwork src={artwork} title={title} kind={kind} className="size-full transition-transform duration-500 group-hover/card:scale-[1.03]" />
        <div className="absolute inset-0 bg-gradient-to-t from-black/75 via-transparent to-transparent" />
        {badge && <div className="absolute top-2 left-2">{badge}</div>}
        {onPlay && (
          <div className="absolute inset-0 grid place-items-center opacity-0 transition duration-200 group-hover/card:opacity-100">
            <PlayFab onPlay={onPlay} />
          </div>
        )}
        {actions && (
          <div className="absolute top-1.5 right-1.5 flex gap-0.5 opacity-0 transition-opacity group-hover/card:opacity-100 [&_button]:text-white/85">
            {actions}
          </div>
        )}
        <ProgressLine value={progress} className="absolute bottom-0 left-0" />
      </div>
      <div className="mt-2.5 px-0.5">
        <div className="truncate text-[14px] font-medium">{title}</div>
        {subtitle && <div className="mt-0.5 truncate text-[12.5px] text-muted">{subtitle}</div>}
      </div>
    </div>
  );
}
