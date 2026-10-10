import { useEffect, useRef, useState } from "react";
import { useLocation, useNavigate, useSearchParams } from "react-router";
import { ChevronLeft, ChevronRight, Loader2, Search, X } from "lucide-react";
import { Tip } from "@/components/ui/misc";
import { cn } from "@/lib/utils";
import { useLibraryStore } from "@/stores/libraryStore";

function ScanIndicator() {
  const scan = useLibraryStore((s) => s.scan);
  const navigate = useNavigate();
  if (!scan?.active) return null;
  const label =
    scan.phase === "discovering"
      ? "Looking for media…"
      : scan.phase === "artwork"
        ? "Preparing artwork…"
        : `Indexing ${scan.processed.toLocaleString()} of ${scan.total.toLocaleString()}`;
  return (
    <button
      type="button"
      onClick={() => navigate("/settings")}
      className="flex h-9 items-center gap-2 rounded-full bg-white/[0.06] px-3.5 text-[12.5px] font-medium text-muted transition-colors hover:text-fg"
    >
      <Loader2 className="size-3.5 animate-spin text-accent" />
      {label}
    </button>
  );
}

export function TopBar({ scrolled }: { scrolled: boolean }) {
  const navigate = useNavigate();
  const location = useLocation();
  const [params] = useSearchParams();
  const onSearch = location.pathname === "/search";
  const [value, setValue] = useState(onSearch ? (params.get("q") ?? "") : "");
  const inputRef = useRef<HTMLInputElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    if (!onSearch) setValue("");
  }, [onSearch]);

  const update = (q: string) => {
    setValue(q);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      if (q.trim()) navigate(`/search?q=${encodeURIComponent(q)}`, { replace: onSearch });
      else if (onSearch) navigate("/search", { replace: true });
    }, 180);
  };

  const navBtn = "grid size-9 place-items-center rounded-full bg-black/30 text-fg transition hover:bg-black/50 disabled:opacity-40";

  return (
    <div
      className={cn(
        "sticky top-0 z-30 flex h-16 items-center gap-3 px-8 transition-colors duration-300",
        scrolled ? "bg-bg/85 backdrop-blur-xl" : "bg-transparent",
      )}
    >
      <Tip label="Go back" side="bottom">
        <button type="button" className={navBtn} onClick={() => navigate(-1)} aria-label="Back">
          <ChevronLeft className="size-5" />
        </button>
      </Tip>
      <Tip label="Go forward" side="bottom">
        <button type="button" className={navBtn} onClick={() => navigate(1)} aria-label="Forward">
          <ChevronRight className="size-5" />
        </button>
      </Tip>

      <div className="relative ml-2 w-full max-w-[420px]">
        <Search className="pointer-events-none absolute top-1/2 left-3.5 size-[18px] -translate-y-1/2 text-muted" />
        <input
          id="global-search"
          ref={inputRef}
          value={value}
          onChange={(e) => update(e.target.value)}
          onFocus={() => {
            if (!onSearch) navigate("/search" + (value ? `?q=${encodeURIComponent(value)}` : ""));
          }}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              update("");
              inputRef.current?.blur();
            }
          }}
          placeholder="Search songs, artists, movies, shows…"
          className="h-10 w-full rounded-full bg-white/[0.07] pr-10 pl-10 text-[14px] text-fg outline-none ring-1 ring-transparent transition placeholder:text-subtle hover:bg-white/[0.09] focus:bg-white/[0.1] focus:ring-white/20"
        />
        {value ? (
          <button
            type="button"
            aria-label="Clear search"
            onClick={() => update("")}
            className="absolute top-1/2 right-3 -translate-y-1/2 rounded-full p-1 text-muted hover:text-fg"
          >
            <X className="size-4" />
          </button>
        ) : (
          <kbd className="pointer-events-none absolute top-1/2 right-3.5 -translate-y-1/2 rounded border border-line-strong px-1.5 text-[10.5px] text-subtle">
            Ctrl K
          </kbd>
        )}
      </div>

      <div className="ml-auto flex items-center gap-2">
        <ScanIndicator />
      </div>
    </div>
  );
}
