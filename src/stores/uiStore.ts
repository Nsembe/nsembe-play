import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { EngineName } from "@/types";

interface UiState {
  sidebarCollapsed: boolean;
  queueOpen: boolean;
  nowPlayingOpen: boolean;
  /** Hides the app chrome (fullscreen built-in video). */
  immersive: boolean;
  engines: { audio: EngineName; video: EngineName };
  createPlaylist: { open: boolean; mediaIds: number[] };
  toggleSidebar: () => void;
  setQueueOpen: (open: boolean) => void;
  setNowPlayingOpen: (open: boolean) => void;
  setImmersive: (immersive: boolean) => void;
  setEngines: (engines: { audio: EngineName; video: EngineName }) => void;
  openCreatePlaylist: (mediaIds?: number[]) => void;
  closeCreatePlaylist: () => void;
}

export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      sidebarCollapsed: false,
      queueOpen: false,
      nowPlayingOpen: false,
      immersive: false,
      engines: { audio: "builtin", video: "builtin" },
      createPlaylist: { open: false, mediaIds: [] },
      toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
      setQueueOpen: (queueOpen) => set({ queueOpen }),
      setNowPlayingOpen: (nowPlayingOpen) => set({ nowPlayingOpen }),
      setImmersive: (immersive) => set({ immersive }),
      setEngines: (engines) => set({ engines }),
      openCreatePlaylist: (mediaIds = []) => set({ createPlaylist: { open: true, mediaIds } }),
      closeCreatePlaylist: () => set({ createPlaylist: { open: false, mediaIds: [] } }),
    }),
    {
      name: "nsembe-ui",
      partialize: (s) => ({ sidebarCollapsed: s.sidebarCollapsed }),
    },
  ),
);
