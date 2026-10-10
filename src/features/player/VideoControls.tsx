// Small menus shared by both video player front-ends.

import { Captions, Gauge, Languages } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Tip } from "@/components/ui/misc";

export const SPEEDS = [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2];

const btn = "grid size-10 place-items-center rounded-full text-white/85 transition hover:bg-white/10 hover:text-white";

export function SpeedMenu({ speed, onChange }: { speed: number; onChange: (s: number) => void }) {
  return (
    <DropdownMenu>
      <Tip label="Playback speed">
        <DropdownMenuTrigger asChild>
          <button type="button" className={btn}>
            <Gauge className="size-5" />
          </button>
        </DropdownMenuTrigger>
      </Tip>
      <DropdownMenuContent side="top" align="end">
        <DropdownMenuLabel>Playback speed</DropdownMenuLabel>
        <DropdownMenuRadioGroup value={String(speed)} onValueChange={(v) => onChange(Number(v))}>
          {SPEEDS.map((s) => (
            <DropdownMenuRadioItem key={s} value={String(s)}>
              {s === 1 ? "Normal" : `${s}×`}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export interface TrackOption {
  id: string;
  label: string;
}

export function TrackMenu({
  kind,
  options,
  value,
  onChange,
  allowOff,
}: {
  kind: "subtitles" | "audio";
  options: TrackOption[];
  value: string;
  onChange: (id: string) => void;
  allowOff?: boolean;
}) {
  const Icon = kind === "subtitles" ? Captions : Languages;
  const label = kind === "subtitles" ? "Subtitles" : "Audio";
  return (
    <DropdownMenu>
      <Tip label={label}>
        <DropdownMenuTrigger asChild>
          <button type="button" className={btn} disabled={options.length === 0 && !allowOff}>
            <Icon className="size-5" />
          </button>
        </DropdownMenuTrigger>
      </Tip>
      <DropdownMenuContent side="top" align="end" className="max-h-80 overflow-y-auto">
        <DropdownMenuLabel>{label}</DropdownMenuLabel>
        <DropdownMenuRadioGroup value={value} onValueChange={onChange}>
          {allowOff && <DropdownMenuRadioItem value="off">Off</DropdownMenuRadioItem>}
          {options.map((o) => (
            <DropdownMenuRadioItem key={o.id} value={o.id}>
              {o.label}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
        {options.length === 0 && <div className="px-2.5 py-2 text-[12.5px] text-muted">None available</div>}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export function languageLabel(code: string | null, title: string | null, fallback: string): string {
  let lang: string | null = null;
  if (code) {
    try {
      lang = new Intl.DisplayNames(undefined, { type: "language" }).of(code.length === 3 ? code.slice(0, 2) : code) ?? code;
    } catch {
      lang = code;
    }
  }
  return [lang, title].filter(Boolean).join(" · ") || fallback;
}

export { btn as videoButtonClass };
