import { useEffect } from "react";
import { useLocation, useNavigate } from "react-router";
import { isTypingTarget } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";
import { useUiStore } from "@/stores/uiStore";

/**
 * App-wide keyboard shortcuts:
 *  Space          play / pause music
 *  Ctrl+K or /    focus search
 *  Ctrl+→ / ←     next / previous track
 *  Ctrl+↑ / ↓     volume up / down
 *  M              mute
 *  Ctrl+,         settings
 *  Esc            close Now Playing / queue
 * The built-in video player handles its own keys while it is open.
 */
export function useGlobalShortcuts() {
  const navigate = useNavigate();
  const location = useLocation();

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const onWatch = location.pathname.startsWith("/watch/");
      const player = usePlayerStore.getState();
      const ui = useUiStore.getState();
      const mod = e.ctrlKey || e.metaKey;

      if ((mod && e.key.toLowerCase() === "k") || (e.key === "/" && !isTypingTarget(e.target))) {
        e.preventDefault();
        document.getElementById("global-search")?.focus();
        return;
      }
      if (mod && e.key === ",") {
        e.preventDefault();
        navigate("/settings");
        return;
      }
      if (isTypingTarget(e.target) || onWatch) return;

      if (e.key === "Escape") {
        if (ui.nowPlayingOpen) ui.setNowPlayingOpen(false);
        else if (ui.queueOpen) ui.setQueueOpen(false);
        return;
      }
      if (e.key === " " && !mod) {
        const el = e.target as HTMLElement;
        if (el.tagName === "BUTTON" || el.getAttribute("role") === "slider") return;
        e.preventDefault();
        void player.toggle();
      } else if (mod && e.key === "ArrowRight") {
        e.preventDefault();
        void player.next();
      } else if (mod && e.key === "ArrowLeft") {
        e.preventDefault();
        void player.previous();
      } else if (mod && e.key === "ArrowUp") {
        e.preventDefault();
        player.setVolume(player.volume + 5);
      } else if (mod && e.key === "ArrowDown") {
        e.preventDefault();
        player.setVolume(player.volume - 5);
      } else if (!mod && e.key.toLowerCase() === "m") {
        player.toggleMute();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [navigate, location.pathname]);
}
