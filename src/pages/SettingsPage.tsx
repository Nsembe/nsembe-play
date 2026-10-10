import { useState, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  CheckCircle2,
  Clapperboard,
  FolderPlus,
  HardDrive,
  Loader2,
  Music2,
  RefreshCw,
  ShieldCheck,
  Trash2,
  Tv,
  XCircle,
} from "lucide-react";
import { toast } from "sonner";
import { PageHeader } from "@/components/EmptyState";
import { Button } from "@/components/ui/button";
import { Input, Switch } from "@/components/ui/misc";
import { pickAndAddFolder } from "@/features/library/folderPicker";
import { ConfirmDialog } from "@/features/playlists/PlaylistDialogs";
import { api } from "@/lib/api";
import { cn, plural, relativeDate } from "@/lib/utils";
import { useLibraryStore } from "@/stores/libraryStore";
import type { FolderKind, LibraryFolder, Settings } from "@/types";

const kindMeta: Record<FolderKind, { label: string; icon: typeof Music2 }> = {
  music: { label: "Music", icon: Music2 },
  movies: { label: "Movies", icon: Clapperboard },
  tv: { label: "TV Shows", icon: Tv },
};

function Card({ title, description, children }: { title: string; description?: string; children: ReactNode }) {
  return (
    <section className="rounded-2xl bg-surface/70 p-6 ring-1 ring-line">
      <h2 className="font-display text-lg font-bold tracking-tight">{title}</h2>
      {description && <p className="mt-1 text-[13.5px] text-muted">{description}</p>}
      <div className="mt-5">{children}</div>
    </section>
  );
}

function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-6 border-t border-line py-4 first:border-t-0 first:pt-0">
      <div className="min-w-0">
        <div className="text-[14px] font-medium">{label}</div>
        {hint && <div className="mt-0.5 text-[12.5px] text-muted">{hint}</div>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function useSettings() {
  const qc = useQueryClient();
  const { data } = useQuery({ queryKey: ["settings"], queryFn: api.settings });
  const mutation = useMutation({
    mutationFn: ({ key, value }: { key: string; value: string | number | boolean }) => api.setSetting(key, value),
    onSuccess: (s) => {
      qc.setQueryData(["settings"], s);
      void qc.invalidateQueries({ queryKey: ["app-status"] });
    },
    onError: (e) => toast.error("Couldn't save setting", { description: (e as Error).message }),
  });
  const set = (key: string, value: string | number | boolean) => mutation.mutate({ key, value });
  return { settings: data as Settings | undefined, set };
}

function ScanProgressBar() {
  const scan = useLibraryStore((s) => s.scan);
  if (!scan?.active) return null;
  const pct = scan.total > 0 ? Math.round((scan.processed / scan.total) * 100) : null;
  return (
    <div className="mb-5 rounded-xl bg-forest/60 p-4 ring-1 ring-accent/20">
      <div className="flex items-center justify-between text-[13px]">
        <span className="flex items-center gap-2 font-medium">
          <Loader2 className="size-4 animate-spin text-accent" />
          {scan.phase === "discovering" ? "Looking for media…" : scan.phase === "artwork" ? "Generating artwork and thumbnails…" : "Indexing your library…"}
        </span>
        {pct !== null && scan.phase === "indexing" && (
          <span className="text-muted tabular-nums">
            {scan.processed.toLocaleString()} / {scan.total.toLocaleString()}
          </span>
        )}
      </div>
      <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-white/10">
        <div
          className={cn("h-full rounded-full bg-accent transition-[width] duration-300", pct === null && "w-1/3 animate-pulse")}
          style={pct !== null ? { width: `${Math.max(pct, 2)}%` } : undefined}
        />
      </div>
      {scan.currentPath && <div className="mt-2 truncate text-[12px] text-muted">{scan.currentPath}</div>}
    </div>
  );
}

function FolderRow({ folder, onRemove }: { folder: LibraryFolder; onRemove: () => void }) {
  const { label, icon: Icon } = kindMeta[folder.kind];
  return (
    <div className="flex items-center gap-4 rounded-xl bg-white/[0.03] p-4">
      <div className="grid size-11 shrink-0 place-items-center rounded-lg bg-forest text-accent">
        <Icon className="size-5" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-[14px] font-medium" title={folder.path}>
            {folder.path}
          </span>
          <span className="shrink-0 rounded-full bg-white/[0.07] px-2 py-0.5 text-[11px] text-muted">{label}</span>
        </div>
        <div className="mt-1 flex flex-wrap gap-x-3 text-[12.5px] text-muted">
          <span>{plural(folder.itemCount, "item")}</span>
          {folder.missingCount > 0 && <span className="text-warning">{folder.missingCount} missing</span>}
          {folder.errorCount > 0 && <span className="text-warning">{folder.errorCount} with problems</span>}
          <span>{folder.lastScannedAt ? `Scanned ${relativeDate(folder.lastScannedAt)}` : "Waiting to scan"}</span>
        </div>
        {(!folder.accessible || folder.lastError) && (
          <div className="mt-1.5 flex items-center gap-1.5 text-[12.5px] text-warning">
            <AlertTriangle className="size-3.5" />
            {folder.lastError ?? "This folder is not accessible right now."}
          </div>
        )}
      </div>
      <Button variant="ghost" size="sm" onClick={() => void api.rescan(folder.id)}>
        <RefreshCw />
        Rescan
      </Button>
      <Button variant="ghost" size="icon-sm" onClick={onRemove} aria-label="Remove folder">
        <Trash2 />
      </Button>
    </div>
  );
}

function LibrarySection() {
  const qc = useQueryClient();
  const { data: folders } = useQuery({ queryKey: ["folders"], queryFn: api.folders, refetchInterval: 5000 });
  const { data: problems } = useQuery({ queryKey: ["problem-files"], queryFn: api.problemFiles });
  const [removing, setRemoving] = useState<LibraryFolder | null>(null);
  const [showProblems, setShowProblems] = useState(false);
  const missing = problems?.filter((p) => p.missing).length ?? 0;

  const add = async (kind: FolderKind) => {
    if (await pickAndAddFolder(kind)) void qc.invalidateQueries();
  };

  return (
    <Card title="Media Library" description="Nsembe Play reads your files where they are. Nothing is copied, moved or uploaded.">
      <ScanProgressBar />
      <div className="flex flex-col gap-2.5">
        {folders?.map((f) => <FolderRow key={f.id} folder={f} onRemove={() => setRemoving(f)} />)}
        {folders?.length === 0 && <p className="text-[13.5px] text-muted">No folders yet. Add one to start building your library.</p>}
      </div>
      <div className="mt-5 flex flex-wrap gap-2">
        {(Object.keys(kindMeta) as FolderKind[]).map((k) => (
          <Button key={k} variant="secondary" onClick={() => void add(k)}>
            <FolderPlus />
            Add {k === "tv" ? "TV" : k} folder
          </Button>
        ))}
      </div>
      <div className="mt-6 flex flex-wrap gap-2 border-t border-line pt-5">
        <Button variant="ghost" size="sm" onClick={() => void api.rescan(undefined, false)}>
          <RefreshCw />
          Scan for changes
        </Button>
        <Button variant="ghost" size="sm" onClick={() => void api.rescan(undefined, true).then(() => toast.info("Re-reading metadata for every file…"))}>
          <RefreshCw />
          Refresh all metadata
        </Button>
        {missing > 0 && (
          <Button
            variant="ghost"
            size="sm"
            onClick={async () => {
              const n = await api.cleanupMissing();
              toast.success(`Removed ${plural(n, "missing item")}`);
              void qc.invalidateQueries();
            }}
          >
            <Trash2 />
            Forget {plural(missing, "missing file")}
          </Button>
        )}
        {problems && problems.length > 0 && (
          <Button variant="ghost" size="sm" onClick={() => setShowProblems(!showProblems)}>
            <AlertTriangle />
            {showProblems ? "Hide" : "Show"} {plural(problems.length, "file")} with problems
          </Button>
        )}
      </div>
      {showProblems && problems && (
        <div className="mt-4 max-h-72 overflow-y-auto rounded-xl bg-black/20 p-2">
          {problems.map((p) => (
            <div key={p.mediaId} className="flex items-start gap-3 rounded-lg px-3 py-2 text-[12.5px]">
              {p.missing ? <XCircle className="mt-0.5 size-4 shrink-0 text-danger" /> : <AlertTriangle className="mt-0.5 size-4 shrink-0 text-warning" />}
              <div className="min-w-0">
                <div className="truncate text-fg/90" title={p.path}>
                  {p.path}
                </div>
                <div className="text-muted">{p.missing ? "File not found. It may have been moved or deleted." : p.error}</div>
              </div>
            </div>
          ))}
        </div>
      )}
      <ConfirmDialog
        open={!!removing}
        onOpenChange={(o) => !o && setRemoving(null)}
        title="Remove this folder?"
        description={`${removing?.path ?? ""} will be removed from your library. The files on disk are not touched.`}
        confirmLabel="Remove"
        onConfirm={async () => {
          if (removing) await api.removeFolder(removing.id);
          setRemoving(null);
          void qc.invalidateQueries();
        }}
      />
    </Card>
  );
}

function EngineSelect({ value, onChange, mpvFound }: { value: string; onChange: (v: string) => void; mpvFound: boolean }) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="h-9 rounded-lg bg-white/[0.07] px-3 text-[13px] text-fg outline-none"
    >
      <option value="auto">Automatic {mpvFound ? "(mpv)" : "(built-in)"}</option>
      <option value="mpv" disabled={!mpvFound}>
        mpv {mpvFound ? "" : "(not installed)"}
      </option>
      <option value="builtin">Built-in player</option>
    </select>
  );
}

function ToolRow({ name, settingKey, found, settings, onSet }: { name: string; settingKey: string; found: string | null; settings: Settings; onSet: (k: string, v: string) => void }) {
  const [value, setValue] = useState(String(settings[settingKey] ?? ""));
  return (
    <div className="border-t border-line py-4 first:border-t-0 first:pt-0">
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-2 text-[14px] font-medium">
          {found ? <CheckCircle2 className="size-4 text-accent" /> : <XCircle className="size-4 text-subtle" />}
          {name}
        </div>
        <span className={cn("truncate text-[12.5px]", found ? "text-muted" : "text-subtle")} title={found ?? undefined}>
          {found ?? "Not found"}
        </span>
      </div>
      <div className="mt-2.5 flex gap-2">
        <Input
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onBlur={() => value !== settings[settingKey] && onSet(settingKey, value.trim())}
          placeholder={`Custom path to ${name.toLowerCase()}.exe (optional)`}
          className="h-9"
        />
        <Button
          variant="ghost"
          size="sm"
          className="h-9"
          onClick={async () => {
            const file = await open({ multiple: false, directory: false, title: `Locate ${name}`, filters: [{ name, extensions: ["exe", "*"] }] });
            if (typeof file === "string") {
              setValue(file);
              onSet(settingKey, file);
            }
          }}
        >
          Browse
        </Button>
      </div>
    </div>
  );
}

export function SettingsPage() {
  const qc = useQueryClient();
  const { settings, set } = useSettings();
  const { data: status } = useQuery({ queryKey: ["app-status"], queryFn: api.appStatus });
  const mpvFound = !!status?.tools.mpv;

  return (
    <div>
      <PageHeader title="Settings" />
      <div className="flex max-w-4xl flex-col gap-6 px-8">
        <LibrarySection />

        {settings && (
          <Card title="Playback">
            <Row label="Music player" hint="mpv supports virtually every audio format. The built-in player covers MP3, FLAC, AAC, OGG and WAV.">
              <EngineSelect value={String(settings["player.audioEngine"])} onChange={(v) => set("player.audioEngine", v)} mpvFound={mpvFound} />
            </Row>
            <Row label="Video player" hint="mpv opens videos in its own window with full subtitle and audio-track support. The built-in player plays MP4/WebM inside the app.">
              <EngineSelect value={String(settings["player.videoEngine"])} onChange={(v) => set("player.videoEngine", v)} mpvFound={mpvFound} />
            </Row>
            <Row label="Autoplay next episode" hint="Continue with the next episode when one finishes.">
              <Switch checked={!!settings["player.autoplayNextEpisode"]} onCheckedChange={(v) => set("player.autoplayNextEpisode", v)} />
            </Row>
            <Row label="Open videos fullscreen" hint="Applies to the mpv video window.">
              <Switch checked={!!settings["player.videoFullscreen"]} onCheckedChange={(v) => set("player.videoFullscreen", v)} />
            </Row>
            <Row label="Preferred audio language" hint="Comma-separated language codes, e.g. eng,en">
              <Input
                className="h-9 w-40"
                defaultValue={String(settings["player.audioLanguage"] ?? "")}
                onBlur={(e) => set("player.audioLanguage", e.target.value.trim())}
                placeholder="eng"
              />
            </Row>
            <Row label="Preferred subtitle language" hint="Leave empty to keep subtitles off by default.">
              <Input
                className="h-9 w-40"
                defaultValue={String(settings["player.subtitleLanguage"] ?? "")}
                onBlur={(e) => set("player.subtitleLanguage", e.target.value.trim())}
                placeholder="eng"
              />
            </Row>
          </Card>
        )}

        {settings && status && (
          <Card
            title="Media tools"
            description="Optional helpers for playback (mpv), metadata (ffprobe) and thumbnails (ffmpeg). They are detected automatically from the app folder, PATH and common install locations."
          >
            <ToolRow name="mpv" settingKey="tools.mpvPath" found={status.tools.mpv} settings={settings} onSet={set} />
            <ToolRow name="FFprobe" settingKey="tools.ffprobePath" found={status.tools.ffprobe} settings={settings} onSet={set} />
            <ToolRow name="FFmpeg" settingKey="tools.ffmpegPath" found={status.tools.ffmpeg} settings={settings} onSet={set} />
            <Button
              variant="ghost"
              size="sm"
              className="mt-2"
              onClick={async () => {
                const t = await api.refreshTools();
                void qc.invalidateQueries({ queryKey: ["app-status"] });
                const found = [t.mpv && "mpv", t.ffprobe && "ffprobe", t.ffmpeg && "ffmpeg"].filter(Boolean);
                toast.info(found.length ? `Found ${found.join(", ")}` : "No media tools found");
              }}
            >
              <RefreshCw />
              Detect again
            </Button>
          </Card>
        )}

        {settings && (
          <Card title="Library">
            <Row label="Watch folders for changes" hint="New, renamed and deleted files are picked up automatically.">
              <Switch checked={!!settings["library.watchFolders"]} onCheckedChange={(v) => set("library.watchFolders", v)} />
            </Row>
            <Row label="Generate video thumbnails" hint="Uses FFmpeg in the background to create backdrops and episode stills.">
              <Switch checked={!!settings["library.videoThumbnails"]} onCheckedChange={(v) => set("library.videoThumbnails", v)} />
            </Row>
          </Card>
        )}

        <Card title="Privacy">
          <div className="flex gap-4">
            <ShieldCheck className="size-6 shrink-0 text-accent" />
            <div className="text-[13.5px] leading-relaxed text-muted">
              Nsembe Play works completely offline. There is no account, no cloud and no tracking. Your library database and artwork cache live on
              this device:
              <div className="mt-2 flex items-center gap-2 rounded-lg bg-black/25 px-3 py-2 font-mono text-[12px] text-fg/80">
                <HardDrive className="size-4 shrink-0 text-muted" />
                <span className="truncate">{status?.dataDir}</span>
              </div>
            </div>
          </div>
        </Card>

        <Card title="About">
          <div className="flex items-center gap-4">
            <img src="/icon.svg" alt="" className="size-14 rounded-xl" />
            <div>
              <div className="font-display text-lg font-bold">Nsembe Play</div>
              <div className="text-[13px] text-muted">Version {status?.version} · Your media. Your library. Your device.</div>
            </div>
          </div>
          <div className="mt-6 grid grid-cols-2 gap-x-8 gap-y-2 text-[13px]">
            {[
              ["Play / pause", "Space"],
              ["Search", "Ctrl K"],
              ["Next / previous song", "Ctrl → / ←"],
              ["Volume", "Ctrl ↑ / ↓"],
              ["Mute", "M"],
              ["Settings", "Ctrl ,"],
              ["Next / previous episode (mpv)", "N / P"],
              ["Fullscreen (video)", "F"],
            ].map(([a, k]) => (
              <div key={a} className="flex justify-between border-b border-line py-1.5">
                <span className="text-muted">{a}</span>
                <kbd className="text-fg/80">{k}</kbd>
              </div>
            ))}
          </div>
        </Card>
      </div>
    </div>
  );
}
