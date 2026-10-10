import type { ReactNode } from "react";
import { Play, Shuffle } from "lucide-react";
import { Tip } from "@/components/ui/misc";
import { assetUrl } from "@/lib/media";
import { hueFrom } from "@/lib/utils";

/** Spotify-style header for albums, artists, playlists and genres. */
export function CollectionHeader({
  cover,
  eyebrow,
  title,
  meta,
  description,
  backdrop,
  seed,
}: {
  cover: ReactNode;
  eyebrow: string;
  title: string;
  meta: ReactNode;
  description?: string | null;
  backdrop?: string | null;
  seed: string;
}) {
  const hue = hueFrom(seed);
  const bg = assetUrl(backdrop);
  return (
    <header className="relative overflow-hidden">
      <div
        className="absolute inset-0"
        style={{ background: `linear-gradient(180deg, hsl(${hue} 35% 22%) 0%, hsl(${hue} 30% 12%) 60%, var(--color-bg) 100%)` }}
      />
      {bg && <img src={bg} alt="" className="absolute inset-0 size-full scale-125 object-cover opacity-35 blur-3xl" />}
      <div className="absolute inset-0 bg-gradient-to-b from-transparent via-transparent to-bg" />
      <div className="relative flex animate-rise items-end gap-7 px-8 pt-24 pb-7">
        <div className="shrink-0 shadow-[0_24px_60px_-18px_rgb(0_0_0/0.85)]">{cover}</div>
        <div className="min-w-0 pb-1">
          <div className="text-[12px] font-bold tracking-[0.18em] text-fg/80 uppercase">{eyebrow}</div>
          <h1 className="mt-2 line-clamp-2 font-display text-[clamp(2rem,5vw,4.2rem)] leading-[1.02] font-extrabold tracking-tight text-balance">{title}</h1>
          {description && <p className="mt-3 line-clamp-2 max-w-2xl text-[14px] text-fg/70">{description}</p>}
          <div className="mt-3 flex flex-wrap items-center gap-x-1.5 text-[14px] text-fg/80">{meta}</div>
        </div>
      </div>
    </header>
  );
}

/** Big play + shuffle buttons followed by optional extra actions. */
export function CollectionActions({
  onPlay,
  onShuffle,
  children,
  disabled,
}: {
  onPlay: () => void;
  onShuffle?: () => void;
  children?: ReactNode;
  disabled?: boolean;
}) {
  return (
    <div className="flex items-center gap-4 px-8 py-5">
      <Tip label="Play">
        <button
          type="button"
          disabled={disabled}
          onClick={onPlay}
          className="grid size-14 place-items-center rounded-full bg-accent text-forest shadow-[0_10px_30px_-8px_rgb(120_247_96/0.6)] transition hover:scale-105 hover:bg-accent-hover active:scale-95 disabled:opacity-40"
        >
          <Play className="size-6 translate-x-px fill-current" />
        </button>
      </Tip>
      {onShuffle && (
        <Tip label="Shuffle">
          <button type="button" disabled={disabled} onClick={onShuffle} className="grid size-10 place-items-center rounded-full text-muted transition hover:text-fg disabled:opacity-40">
            <Shuffle className="size-6" />
          </button>
        </Tip>
      )}
      <div className="flex items-center gap-1 [&_svg]:!size-6">{children}</div>
    </div>
  );
}
