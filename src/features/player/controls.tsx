// Building blocks shared by the bottom player bar and the Now Playing view.

import { useState } from "react";
import { Pause, Play, Repeat, Repeat1, Shuffle, SkipBack, SkipForward, Volume1, Volume2, VolumeX } from "lucide-react";
import { Slider } from "@/components/ui/slider";
import { Tip } from "@/components/ui/misc";
import { cn, formatTime } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";

export function SeekBar({ className }: { className?: string }) {
  const position = usePlayerStore((s) => s.position);
  const duration = usePlayerStore((s) => s.duration);
  const seek = usePlayerStore((s) => s.seek);
  const [drag, setDrag] = useState<number | null>(null);
  const shown = drag ?? position;
  return (
    <div className={cn("flex w-full items-center gap-2.5 text-[11.5px] text-muted tabular-nums", className)}>
      <span className="w-10 text-right">{formatTime(shown)}</span>
      <Slider
        aria-label="Seek"
        min={0}
        max={Math.max(duration, 1)}
        step={1}
        value={[Math.min(shown, Math.max(duration, 1))]}
        onValueChange={([v]) => setDrag(v)}
        onValueCommit={([v]) => {
          setDrag(null);
          void seek(v);
        }}
        disabled={!duration}
      />
      <span className="w-10">{formatTime(duration)}</span>
    </div>
  );
}

export function TransportControls({ size = "md" }: { size?: "md" | "lg" }) {
  const paused = usePlayerStore((s) => s.paused);
  const shuffle = usePlayerStore((s) => s.shuffle);
  const repeat = usePlayerStore((s) => s.repeat);
  const hasItem = usePlayerStore((s) => s.queue.length > 0);
  const { toggle, next, previous, toggleShuffle, cycleRepeat } = usePlayerStore.getState();
  const lg = size === "lg";
  const side = cn("grid place-items-center rounded-full text-muted transition hover:text-fg disabled:opacity-40", lg ? "size-11" : "size-8");
  const iconSide = lg ? "size-6" : "size-[18px]";

  return (
    <div className={cn("flex items-center justify-center", lg ? "gap-6" : "gap-4")}>
      <Tip label={shuffle ? "Disable shuffle" : "Enable shuffle"}>
        <button type="button" onClick={toggleShuffle} className={cn(side, "relative", shuffle && "text-accent hover:text-accent-hover")}>
          <Shuffle className={cn(lg ? "size-5" : "size-4")} />
          {shuffle && <span className="absolute bottom-0 size-1 rounded-full bg-accent" />}
        </button>
      </Tip>
      <Tip label="Previous">
        <button type="button" onClick={() => void previous()} disabled={!hasItem} className={side}>
          <SkipBack className={cn(iconSide, "fill-current")} />
        </button>
      </Tip>
      <Tip label={paused ? "Play" : "Pause"}>
        <button
          type="button"
          onClick={() => void toggle()}
          disabled={!hasItem}
          className={cn(
            "grid place-items-center rounded-full bg-fg text-bg transition hover:scale-105 active:scale-95 disabled:opacity-40",
            lg ? "size-16" : "size-9",
          )}
        >
          {paused ? (
            <Play className={cn(lg ? "size-7" : "size-[18px]", "translate-x-px fill-current")} />
          ) : (
            <Pause className={cn(lg ? "size-7" : "size-[18px]", "fill-current")} />
          )}
        </button>
      </Tip>
      <Tip label="Next">
        <button type="button" onClick={() => void next()} disabled={!hasItem} className={side}>
          <SkipForward className={cn(iconSide, "fill-current")} />
        </button>
      </Tip>
      <Tip label={repeat === "off" ? "Repeat all" : repeat === "all" ? "Repeat one" : "Repeat off"}>
        <button type="button" onClick={cycleRepeat} className={cn(side, "relative", repeat !== "off" && "text-accent hover:text-accent-hover")}>
          {repeat === "one" ? <Repeat1 className={cn(lg ? "size-5" : "size-4")} /> : <Repeat className={cn(lg ? "size-5" : "size-4")} />}
          {repeat !== "off" && <span className="absolute bottom-0 size-1 rounded-full bg-accent" />}
        </button>
      </Tip>
    </div>
  );
}

export function VolumeControl({ className }: { className?: string }) {
  const volume = usePlayerStore((s) => s.volume);
  const muted = usePlayerStore((s) => s.muted);
  const { setVolume, toggleMute } = usePlayerStore.getState();
  const Icon = muted || volume === 0 ? VolumeX : volume < 50 ? Volume1 : Volume2;
  return (
    <div className={cn("flex items-center gap-2", className)}>
      <Tip label={muted ? "Unmute" : "Mute"}>
        <button type="button" onClick={toggleMute} className="grid size-8 place-items-center rounded-full text-muted hover:text-fg">
          <Icon className="size-[18px]" />
        </button>
      </Tip>
      <Slider
        aria-label="Volume"
        className="w-24"
        min={0}
        max={100}
        step={1}
        value={[muted ? 0 : volume]}
        onValueChange={([v]) => setVolume(v)}
      />
    </div>
  );
}
