import { useEffect, useRef, useState, type ReactNode } from "react";
import { Link } from "react-router";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { cn } from "@/lib/utils";

/** Horizontally scrolling row of cards with a heading. */
export function Shelf({
  title,
  to,
  subtitle,
  children,
  className,
}: {
  title: string;
  to?: string;
  subtitle?: string;
  children: ReactNode;
  className?: string;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [canLeft, setCanLeft] = useState(false);
  const [canRight, setCanRight] = useState(false);

  const update = () => {
    const el = ref.current;
    if (!el) return;
    setCanLeft(el.scrollLeft > 4);
    setCanRight(el.scrollLeft + el.clientWidth < el.scrollWidth - 4);
  };

  useEffect(() => {
    update();
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => ro.disconnect();
  }, [children]);

  const scroll = (dir: 1 | -1) => {
    const el = ref.current;
    if (el) el.scrollBy({ left: dir * el.clientWidth * 0.85, behavior: "smooth" });
  };

  const arrow = "absolute top-[38%] z-10 grid size-10 -translate-y-1/2 place-items-center rounded-full bg-surface-3/90 text-fg shadow-pop opacity-0 backdrop-blur transition hover:bg-surface-3 group-hover/shelf:opacity-100";

  return (
    <section className={cn("group/shelf animate-rise", className)}>
      <div className="mb-3 flex items-end justify-between gap-4 px-8">
        <div>
          <h2 className="font-display text-[21px] font-bold tracking-tight">
            {to ? (
              <Link to={to} className="hover:underline underline-offset-4">
                {title}
              </Link>
            ) : (
              title
            )}
          </h2>
          {subtitle && <p className="mt-0.5 text-sm text-muted">{subtitle}</p>}
        </div>
        {to && (
          <Link to={to} className="shrink-0 text-[13px] font-semibold text-muted transition-colors hover:text-fg">
            Show all
          </Link>
        )}
      </div>
      <div className="relative">
        <div
          ref={ref}
          onScroll={update}
          className="flex snap-x scroll-px-8 gap-3 overflow-x-auto scroll-smooth px-8 pb-2 scrollbar-none [&>*]:snap-start"
        >
          {children}
        </div>
        {canLeft && (
          <button type="button" aria-label="Scroll left" onClick={() => scroll(-1)} className={cn(arrow, "left-3")}>
            <ChevronLeft className="size-5" />
          </button>
        )}
        {canRight && (
          <button type="button" aria-label="Scroll right" onClick={() => scroll(1)} className={cn(arrow, "right-3")}>
            <ChevronRight className="size-5" />
          </button>
        )}
      </div>
    </section>
  );
}
