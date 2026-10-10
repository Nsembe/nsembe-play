import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { api } from "@/lib/api";
import type { FolderKind, LibraryFolder } from "@/types";

const titles: Record<FolderKind, string> = {
  music: "Choose your music folder",
  movies: "Choose your movies folder",
  tv: "Choose your TV shows folder",
};

/**
 * Shows the native folder picker and registers the chosen folder. The path is
 * validated and indexed by Rust; React never reads the filesystem itself.
 */
export async function pickAndAddFolder(kind: FolderKind): Promise<LibraryFolder | null> {
  const selected = await open({ directory: true, multiple: false, title: titles[kind] });
  if (!selected || Array.isArray(selected)) return null;
  try {
    const folder = await api.addFolder(selected, kind);
    toast.success("Folder added", { description: `Scanning ${folder.path}…` });
    return folder;
  } catch (e) {
    toast.error("Couldn't add folder", { description: (e as Error).message });
    return null;
  }
}
