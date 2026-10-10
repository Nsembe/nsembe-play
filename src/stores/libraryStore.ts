import { create } from "zustand";
import type { ScanProgress } from "@/types";

interface LibraryState {
  scan: ScanProgress | null;
  setScan: (scan: ScanProgress) => void;
}

export const useLibraryStore = create<LibraryState>()((set) => ({
  scan: null,
  setScan: (scan) => set({ scan }),
}));
