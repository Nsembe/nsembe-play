import { convertFileSrc } from "@tauri-apps/api/core";

const cache = new Map<string, string>();

/** Converts a local artwork/media path into a URL the webview can load (asset protocol). */
export function assetUrl(path: string | null | undefined): string | undefined {
  if (!path) return undefined;
  let url = cache.get(path);
  if (!url) {
    url = convertFileSrc(path);
    cache.set(path, url);
  }
  return url;
}
