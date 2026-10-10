import { useParams, useSearchParams } from "react-router";
import { BuiltinVideoPlayer } from "@/features/player/BuiltinVideoPlayer";
import { MpvVideoRemote } from "@/features/player/MpvVideoRemote";
import { useUiStore } from "@/stores/uiStore";

export function WatchPage() {
  const mediaId = Number(useParams().mediaId);
  const [params] = useSearchParams();
  const engine = useUiStore((s) => s.engines.video);
  const t = params.get("t");
  if (engine === "mpv") return <MpvVideoRemote mediaId={mediaId} />;
  return <BuiltinVideoPlayer key={mediaId} mediaId={mediaId} start={t != null ? Number(t) : null} />;
}
