import { useNavigate } from "react-router";
import { ChevronDown } from "lucide-react";
import { Artwork } from "@/components/Artwork";
import { FavoriteButton } from "@/components/MediaActions";
import { assetUrl } from "@/lib/media";
import { cn, formatTime } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";
import { useUiStore } from "@/stores/uiStore";
import { SeekBar, TransportControls, VolumeControl } from "./controls";

/** Expanded, immersive view of the current song with the upcoming queue. */
export function NowPlaying() {
  const navigate = useNavigate();
  const open = useUiStore((s) => s.nowPlayingOpen);
  const setOpen = useUiStore((s) => s.setNowPlayingOpen);
  const queue = usePlayerStore((s) => s.queue);
  const index = usePlayerStore((s) => s.index);
  const current = queue[index];

  if (!open || !current) return null;
  const bg = assetUrl(current.backdrop ?? current.artwork);
  const upcoming = queue.slice(index + 1, index + 30);

  const go = (path: string) => {
    setOpen(false);
    navigate(path);
  };

  return (
    <div className="fixed inset-x-0 top-0 bottom-[84px] z-40 animate-fade-in overflow-hidden bg-bg">
      {bg && <img src={bg} alt="" className="absolute inset-0 size-full scale-125 object-cover opacity-40 blur-[90px]" />}
      <div className="absolute inset-0 bg-gradient-to-b from-bg/40 via-bg/70 to-bg" />

      <div className="relative flex h-full flex-col">
        <div className="flex h-16 items-center justify-between px-8">
          <button
            type="button"
            onClick={() => setOpen(false)}
            className="grid size-10 place-items-center rounded-full bg-black/30 text-fg hover:bg-black/50"
            aria-label="Close Now Playing"
          >
            <ChevronDown className="size-5" />
          </button>
          <span className="text-[12px] font-semibold tracking-[0.18em] text-muted uppercase">Now Playing</span>
          <div className="size-10" />
        </div>

        <div className="grid min-h-0 flex-1 grid-cols-1 gap-12 px-12 pb-10 lg:grid-cols-[minmax(0,1fr)_380px]">
          <div className="flex min-h-0 flex-col items-center justify-center gap-8">
            <Artwork
              src={current.backdrop ?? current.artwork}
              title={current.albumTitle ?? current.title}
              kind="album"
              eager
              className="aspect-square w-full max-w-[min(440px,48vh)] rounded-xl shadow-[0_30px_80px_-20px_rgb(0_0_0/0.8)]"
            />
            <div className="w-full max-w-[520px]">
              <div className="flex items-start justify-between gap-4">
                <div className="min-w-0">
                  <h1 className="truncate font-display text-3xl font-extrabold tracking-tight">{current.title}</h1>
                  <div className="mt-1 flex items-center gap-2 text-[15px] text-muted">
                    <button type="button" className="truncate hover:text-fg hover:underline" onClick={() => current.artistId && go(`/music/artists/${current.artistId}`)}>
                      {current.subtitle}
                    </button>
                    {current.albumTitle && (
                      <>
                        <span>·</span>
                        <button type="button" className="truncate hover:text-fg hover:underline" onClick={() => current.albumId && go(`/albums/${current.albumId}`)}>
                          {current.albumTitle}
                        </button>
                      </>
                    )}
                  </div>
                </div>
                {current.trackId && <FavoriteButton key={current.trackId} type="track" id={current.trackId} value={current.isFavorite} size="lg" />}
              </div>
              <SeekBar className="mt-6" />
              <div className="mt-4">
                <TransportControls size="lg" />
              </div>
              <div className="mt-6 flex justify-center">
                <VolumeControl />
              </div>
            </div>
          </div>

          <div className="hidden min-h-0 flex-col rounded-2xl bg-black/25 p-5 backdrop-blur-xl lg:flex">
            <h2 className="mb-3 font-display text-lg font-bold">Up next</h2>
            <div className="min-h-0 flex-1 overflow-y-auto">
              {upcoming.length === 0 && <p className="text-sm text-muted">Nothing else in the queue.</p>}
              {upcoming.map((item, i) => (
                <button
                  key={`${item.mediaId}-${i}`}
                  type="button"
                  onDoubleClick={() => void usePlayerStore.getState().jumpTo(index + 1 + i)}
                  className={cn("flex w-full items-center gap-3 rounded-lg px-2 py-1.5 text-left hover:bg-white/[0.06]")}
                >
                  <Artwork src={item.artwork} title={item.albumTitle ?? item.title} kind="album" className="size-10 shrink-0 rounded" />
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[13.5px] font-medium">{item.title}</div>
                    <div className="truncate text-[12px] text-muted">{item.subtitle}</div>
                  </div>
                  <span className="text-[12px] text-muted tabular-nums">{formatTime(item.duration)}</span>
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
