// Typed wrappers around the Tauri commands exposed by the Rust backend.
// React code never calls `invoke` directly; it goes through this module.

import { invoke } from "@tauri-apps/api/core";
import type {
  Album,
  AlbumDetail,
  AppStatus,
  Artist,
  ArtistDetail,
  ContinueItem,
  EngineKind,
  FavoriteType,
  FavoritesOverview,
  FolderKind,
  Genre,
  HomeFeed,
  LibraryFolder,
  Movie,
  MovieDetail,
  Page,
  PlayableItem,
  PlayerCommand,
  PlayerState,
  Playlist,
  PlaylistDetail,
  ProblemFile,
  RecentItem,
  ScanProgress,
  SearchResults,
  Settings,
  Show,
  ShowDetail,
  SubtitleFile,
  ToolStatus,
  Track,
} from "@/types";

export class ApiError extends Error {
  code: string;
  constructor(code: string, message: string) {
    super(message);
    this.code = code;
  }
}

function toApiError(e: unknown): ApiError {
  if (e && typeof e === "object" && "message" in e) {
    const err = e as { code?: string; message: string };
    return new ApiError(err.code ?? "unknown", err.message);
  }
  return new ApiError("unknown", typeof e === "string" ? e : "Something went wrong");
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw toApiError(e);
  }
}

export interface PageArgs {
  offset?: number;
  limit?: number;
  sort?: string;
}

export const api = {
  // App & settings
  appStatus: () => call<AppStatus>("app_status"),
  completeOnboarding: () => call<void>("app_complete_onboarding"),
  settings: () => call<Settings>("settings_get"),
  setSetting: (key: string, value: string | number | boolean) => call<Settings>("settings_set", { key, value }),
  refreshTools: () => call<ToolStatus>("tools_refresh"),
  reveal: (mediaId: number) => call<void>("media_reveal", { mediaId }),

  // Library management
  folders: () => call<LibraryFolder[]>("library_folders"),
  addFolder: (path: string, kind: FolderKind) => call<LibraryFolder>("library_add_folder", { path, kind }),
  removeFolder: (id: number) => call<void>("library_remove_folder", { id }),
  rescan: (folderId?: number, force = false) => call<void>("library_rescan", { folderId: folderId ?? null, force }),
  scanStatus: () => call<ScanProgress>("library_scan_status"),
  cleanupMissing: () => call<number>("library_cleanup_missing"),
  problemFiles: () => call<ProblemFile[]>("library_problem_files"),

  // Browsing
  home: () => call<HomeFeed>("home_feed"),
  recentlyAdded: (a: PageArgs = {}) => call<Page<RecentItem>>("recently_added", { ...a }),
  recentlyPlayed: (kind?: "music" | "video", a: PageArgs = {}) =>
    call<Page<PlayableItem>>("recently_played", { kind: kind ?? null, ...a }),
  continueWatching: () => call<ContinueItem[]>("continue_watching"),
  movies: (a: PageArgs & { genre?: string } = {}) => call<Page<Movie>>("movies_list", { ...a }),
  movieGenres: () => call<string[]>("movie_genres"),
  movie: (id: number) => call<MovieDetail>("movie_detail", { id }),
  shows: (a: PageArgs = {}) => call<Page<Show>>("shows_list", { ...a }),
  show: (id: number) => call<ShowDetail>("show_detail", { id }),
  tracks: (a: PageArgs = {}) => call<Page<Track>>("tracks_list", { ...a }),
  genreTracks: (genre: string, a: PageArgs = {}) => call<Page<Track>>("genre_tracks", { genre, ...a }),
  genres: () => call<Genre[]>("genres_list"),
  albums: (a: PageArgs = {}) => call<Page<Album>>("albums_list", { ...a }),
  album: (id: number) => call<AlbumDetail>("album_detail", { id }),
  artists: (a: PageArgs = {}) => call<Page<Artist>>("artists_list", { ...a }),
  artist: (id: number) => call<ArtistDetail>("artist_detail", { id }),
  favorites: () => call<FavoritesOverview>("favorites_overview"),
  toggleFavorite: (itemType: FavoriteType, itemId: number) => call<boolean>("favorite_toggle", { itemType, itemId }),
  search: (query: string, limit = 8) => call<SearchResults>("search", { query, limit }),
  playables: (mediaIds: number[]) => call<PlayableItem[]>("media_playables", { mediaIds }),
  shuffleMusic: (limit = 200) => call<PlayableItem[]>("music_shuffle", { limit }),
  clearHistory: () => call<void>("history_clear"),
  setWatched: (mediaId: number, watched: boolean) => call<void>("media_set_watched", { mediaId, watched }),

  // Playlists
  playlists: () => call<Playlist[]>("playlists_list"),
  playlist: (id: number) => call<PlaylistDetail>("playlist_detail", { id }),
  createPlaylist: (name: string, description?: string) =>
    call<Playlist>("playlist_create", { name, description: description ?? null }),
  updatePlaylist: (id: number, name: string, description?: string | null) =>
    call<Playlist>("playlist_update", { id, name, description: description ?? null }),
  deletePlaylist: (id: number) => call<void>("playlist_delete", { id }),
  addToPlaylist: (id: number, mediaIds: number[]) => call<number>("playlist_add_items", { id, mediaIds }),
  removeFromPlaylist: (itemId: number) => call<void>("playlist_remove_item", { itemId }),
  reorderPlaylist: (id: number, itemIds: number[]) => call<void>("playlist_reorder", { id, itemIds }),

  // Playback (mpv engine)
  play: (mediaId: number, start?: number) => call<PlayableItem>("player_play", { mediaId, start: start ?? null }),
  playerCommand: (engineKind: EngineKind, command: PlayerCommand) => call<void>("player_command", { engineKind, command }),
  playerState: (engineKind: EngineKind) => call<PlayerState>("player_state", { engineKind }),
  adjacentEpisode: (mediaId: number, forward: boolean) =>
    call<PlayableItem | null>("player_adjacent_episode", { mediaId, forward }),

  // Playback (built-in engine)
  resolve: (mediaId: number) => call<PlayableItem>("playback_resolve", { mediaId }),
  report: (mediaId: number, event: "start" | "progress" | "pause" | "stop" | "ended", position: number, duration?: number | null) =>
    call<boolean>("playback_report", { mediaId, event, position, duration: duration ?? null }),
  subtitles: (mediaId: number) => call<SubtitleFile[]>("media_subtitles", { mediaId }),
};
