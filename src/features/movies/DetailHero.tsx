import type { ReactNode } from "react";
import { Artwork, type ArtworkKind } from "@/components/Artwork";
import { assetUrl } from "@/lib/media";

/** Full-bleed backdrop with poster and title block, used by movie and show pages. */
export function DetailHero({
  backdrop,
  poster,
  title,
  kind,
  eyebrow,
  meta,
  overview,
  actions,
  tags,
}: {
  backdrop: string | null;
  poster: string | null;
  title: string;
  kind: ArtworkKind;
  eyebrow?: string;
  meta: ReactNode;
  overview?: string | null;
  actions: ReactNode;
  tags?: string[];
}) {
  const img = assetUrl(backdrop ?? poster);
  return (
    <section className="relative min-h-[min(72vh,640px)] overflow-hidden">
      {img ? (
        <img src={img} alt="" className="absolute inset-0 size-full animate-fade-in object-cover object-[center_20%]" />
      ) : (
        <div className="absolute inset-0 bg-gradient-to-br from-forest via-surface to-bg" />
      )}
      {!backdrop && img && <div className="absolute inset-0 backdrop-blur-3xl" />}
      <div className="absolute inset-0 bg-gradient-to-r from-bg via-bg/80 to-bg/10" />
      <div className="absolute inset-0 bg-gradient-to-t from-bg via-transparent to-bg/40" />

      <div className="relative flex min-h-[min(72vh,640px)] animate-rise items-end gap-10 px-8 pt-28 pb-12">
        <Artwork
          src={poster}
          title={title}
          kind={kind}
          showTitle
          eager
          className="hidden aspect-[2/3] w-[230px] shrink-0 rounded-xl shadow-[0_30px_60px_-20px_rgb(0_0_0/0.9)] ring-1 ring-white/10 lg:block"
        />
        <div className="max-w-2xl min-w-0">
          {eyebrow && <div className="mb-2 text-[12px] font-bold tracking-[0.2em] text-accent uppercase">{eyebrow}</div>}
          <h1 className="font-display text-[clamp(2.2rem,4.2vw,3.5rem)] leading-[1.03] font-extrabold tracking-tight text-balance">{title}</h1>
          <div className="mt-3 flex flex-wrap items-center gap-x-2.5 gap-y-1 text-[14.5px] font-medium text-fg/80">{meta}</div>
          {tags && tags.length > 0 && (
            <div className="mt-3 flex flex-wrap gap-1.5">
              {tags.map((t) => (
                <span key={t} className="rounded-full bg-white/[0.08] px-2.5 py-0.5 text-[12px] text-fg/80">
                  {t}
                </span>
              ))}
            </div>
          )}
          {overview && <p className="mt-4 line-clamp-4 text-[15px] leading-relaxed text-muted">{overview}</p>}
          <div className="mt-6 flex flex-wrap items-center gap-3">{actions}</div>
        </div>
      </div>
    </section>
  );
}

export function MetaDot() {
  return <span className="text-subtle">•</span>;
}
