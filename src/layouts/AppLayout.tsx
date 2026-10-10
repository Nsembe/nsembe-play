import { useEffect, useRef, useState } from "react";
import { Outlet, useLocation } from "react-router";
import { CreatePlaylistDialog } from "@/features/playlists/PlaylistDialogs";
import { NowPlaying } from "@/features/player/NowPlaying";
import { PlayerBar } from "@/features/player/PlayerBar";
import { QueuePanel } from "@/features/player/QueuePanel";
import { useGlobalShortcuts } from "@/hooks/useGlobalShortcuts";
import { useMediaQuery } from "@/hooks/useMediaQuery";
import { cn } from "@/lib/utils";
import { useUiStore } from "@/stores/uiStore";
import { ScrollContext } from "./scrollContext";
import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";

export function AppLayout() {
  const mainRef = useRef<HTMLElement>(null);
  const location = useLocation();
  const [scrolled, setScrolled] = useState(false);
  const narrow = useMediaQuery("(max-width: 1100px)");
  const userCollapsed = useUiStore((s) => s.sidebarCollapsed);
  const immersive = useUiStore((s) => s.immersive);
  const watching = location.pathname.startsWith("/watch/");
  useGlobalShortcuts();

  // New page: start at the top.
  useEffect(() => {
    mainRef.current?.scrollTo({ top: 0 });
    setScrolled(false);
  }, [location.pathname]);

  return (
    <ScrollContext.Provider value={mainRef}>
      <div className="flex h-full flex-col bg-bg">
        <div className="flex min-h-0 flex-1">
          {!immersive && <Sidebar collapsed={narrow || userCollapsed} />}
          <main
            ref={mainRef}
            onScroll={(e) => setScrolled((e.target as HTMLElement).scrollTop > 8)}
            className={cn("relative min-w-0 flex-1 overflow-x-hidden", watching ? "overflow-hidden" : "overflow-y-auto")}
          >
            {!watching && <TopBar scrolled={scrolled} />}
            <div className={cn(watching ? "h-full" : "-mt-16 pb-16")}>
              <Outlet />
            </div>
          </main>
          {!immersive && <QueuePanel />}
        </div>
        {!immersive && <PlayerBar />}
        <NowPlaying />
        <CreatePlaylistDialog />
      </div>
    </ScrollContext.Provider>
  );
}
