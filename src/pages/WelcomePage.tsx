import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowRight, Check, Clapperboard, Loader2, Music2, Tv } from "lucide-react";
import { Button } from "@/components/ui/button";
import { pickAndAddFolder } from "@/features/library/folderPicker";
import { api } from "@/lib/api";
import { cn, plural } from "@/lib/utils";
import { useLibraryStore } from "@/stores/libraryStore";
import type { FolderKind, LibraryFolder } from "@/types";

const tiles: { kind: FolderKind; title: string; hint: string; icon: typeof Music2 }[] = [
  { kind: "music", title: "Add Music Folder", hint: "Albums, artists and playlists", icon: Music2 },
  { kind: "movies", title: "Add Movies Folder", hint: "Posters, details and resume", icon: Clapperboard },
  { kind: "tv", title: "Add TV Folder", hint: "Shows, seasons and episodes", icon: Tv },
];

function Building({ onDone }: { onDone: () => void }) {
  const scan = useLibraryStore((s) => s.scan);
  const [finished, setFinished] = useState(false);
  const idleChecks = useRef(0);
  const started = useRef(Date.now());
  const { data: status } = useQuery({ queryKey: ["app-status"], queryFn: api.appStatus, refetchInterval: finished ? false : 1200 });

  // Poll the scanner: done once it has been idle for a little while.
  useEffect(() => {
    if (finished) return;
    const t = setInterval(async () => {
      const s = await api.scanStatus();
      if (!s.active && Date.now() - started.current > 1500) {
        idleChecks.current += 1;
        if (idleChecks.current >= 2) setFinished(true);
      } else {
        idleChecks.current = 0;
      }
    }, 700);
    return () => clearInterval(t);
  }, [finished]);

  const stats = status?.stats;
  const pct = scan && scan.total > 0 ? Math.round((scan.processed / scan.total) * 100) : null;
  const statLine = stats
    ? [
        stats.tracks && plural(stats.tracks, "song"),
        stats.albums && plural(stats.albums, "album"),
        stats.movies && plural(stats.movies, "movie"),
        stats.shows && plural(stats.shows, "show"),
      ]
        .filter(Boolean)
        .join(" · ")
    : "";

  return (
    <div className="flex w-full max-w-lg animate-rise flex-col items-center text-center">
      {finished ? (
        <div className="grid size-16 place-items-center rounded-full bg-accent text-forest">
          <Check className="size-8" strokeWidth={3} />
        </div>
      ) : (
        <Loader2 className="size-14 animate-spin text-accent" />
      )}
      <h1 className="mt-6 font-display text-3xl font-extrabold tracking-tight">{finished ? "Your library is ready" : "Building your library..."}</h1>
      <p className="mt-2 text-[15px] text-muted">
        {finished
          ? statLine || "Everything is organised."
          : scan?.phase === "artwork"
            ? "Preparing artwork. You can start exploring already."
            : "Reading your files and organising them. This happens in the background."}
      </p>
      {!finished && (
        <div className="mt-8 w-full">
          <div className="h-1.5 overflow-hidden rounded-full bg-white/10">
            <div
              className={cn("h-full rounded-full bg-accent transition-[width] duration-300", pct === null && "w-1/4 animate-pulse")}
              style={pct !== null ? { width: `${Math.max(pct, 3)}%` } : undefined}
            />
          </div>
          <div className="mt-3 flex justify-between text-[12.5px] text-muted tabular-nums">
            <span className="truncate pr-4">{scan?.currentPath?.split(/[\\/]/).pop() ?? "Looking for media…"}</span>
            {scan && scan.total > 0 && (
              <span className="shrink-0">
                {scan.processed.toLocaleString()} / {scan.total.toLocaleString()}
              </span>
            )}
          </div>
          {statLine && <div className="mt-6 text-[13px] text-fg/80">{statLine}</div>}
        </div>
      )}
      <Button variant={finished ? "primary" : "ghost"} size="lg" className="mt-10" onClick={onDone}>
        {finished ? "Start exploring" : "Continue in the background"}
        <ArrowRight />
      </Button>
    </div>
  );
}

export function WelcomePage() {
  const navigate = useNavigate();
  const qc = useQueryClient();
  const [added, setAdded] = useState<LibraryFolder[]>([]);
  const [step, setStep] = useState<"pick" | "building">("pick");

  const finish = async () => {
    await api.completeOnboarding();
    await qc.invalidateQueries();
    navigate("/", { replace: true });
  };

  return (
    <div className="relative flex h-full items-center justify-center overflow-hidden bg-bg px-8">
      <div className="pointer-events-none absolute -top-40 left-1/2 h-[520px] w-[900px] -translate-x-1/2 rounded-full bg-accent/10 blur-[120px]" />
      <div className="pointer-events-none absolute -bottom-60 -left-40 h-[500px] w-[600px] rounded-full bg-moss/15 blur-[120px]" />

      {step === "pick" ? (
        <div className="relative flex w-full max-w-4xl animate-rise flex-col items-center text-center">
          <img src="/icon.svg" alt="" className="size-20 rounded-2xl shadow-[0_20px_60px_-15px_rgb(120_247_96/0.45)]" />
          <h1 className="mt-8 font-display text-[clamp(2.2rem,5vw,3.4rem)] font-extrabold tracking-tight">
            Welcome to Nsembe <span className="text-accent">Play</span>
          </h1>
          <p className="mt-3 text-lg text-muted">Your media. Your library. Your device.</p>

          <div className="mt-12 grid w-full grid-cols-1 gap-4 sm:grid-cols-3">
            {tiles.map((t) => {
              const folders = added.filter((f) => f.kind === t.kind);
              const done = folders.length > 0;
              return (
                <button
                  key={t.kind}
                  type="button"
                  onClick={async () => {
                    const f = await pickAndAddFolder(t.kind);
                    if (f) setAdded((a) => [...a, f]);
                  }}
                  className={cn(
                    "group flex flex-col items-start rounded-2xl p-6 text-left ring-1 transition duration-200 hover:-translate-y-0.5",
                    done ? "bg-forest/70 ring-accent/40" : "bg-surface/80 ring-line hover:bg-surface-2 hover:ring-line-strong",
                  )}
                >
                  <div className={cn("grid size-12 place-items-center rounded-xl transition", done ? "bg-accent text-forest" : "bg-white/[0.06] text-accent group-hover:bg-accent/15")}>
                    {done ? <Check className="size-6" strokeWidth={3} /> : <t.icon className="size-6" />}
                  </div>
                  <div className="mt-5 font-display text-[17px] font-bold">{t.title}</div>
                  <div className="mt-1 text-[13px] text-muted">{t.hint}</div>
                  {folders.map((f) => (
                    <div key={f.id} className="mt-3 w-full truncate rounded-md bg-black/25 px-2.5 py-1.5 font-mono text-[11.5px] text-fg/80" title={f.path}>
                      {f.path}
                    </div>
                  ))}
                  {done && <div className="mt-2 text-[12px] text-accent">Add another</div>}
                </button>
              );
            })}
          </div>

          <div className="mt-10 flex items-center gap-3">
            <Button variant="ghost" size="lg" onClick={() => void finish()}>
              Skip for now
            </Button>
            <Button variant="primary" size="lg" disabled={added.length === 0} onClick={() => setStep("building")}>
              Continue
              <ArrowRight />
            </Button>
          </div>
          <p className="mt-6 text-[12.5px] text-subtle">Files stay exactly where they are. Nothing is uploaded or copied.</p>
        </div>
      ) : (
        <Building onDone={() => void finish()} />
      )}
    </div>
  );
}
