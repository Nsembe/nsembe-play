// Mirrors the serialized models of the Rust engine (nsembe-core/src/models.rs).

export type FolderKind = "music" | "movies" | "tv";
export type MediaKind = "track" | "movie" | "episode";
export type EngineKind = "audio" | "video";
export type EngineName = "mpv" | "builtin";
export type FavoriteType = "track" | "album" | "artist" | "movie" | "tv_show";

export interface Page<T> {
  items: T[];
  total: number;
  offset: number;
  limit: number;
}

export interface LibraryFolder {
  id: number;
  path: string;
  kind: FolderKind;
  enabled: boolean;
  addedAt: number;
  lastScannedAt: number | null;
  lastError: string | null;
  itemCount: number;
  missingCount: number;
  errorCount: number;
  accessible: boolean;
}

export interface Track {
  id: number;
  mediaId: number;
  title: string;
  artistId: number | null;
  artistName: string | null;
  albumId: number | null;
  albumTitle: string | null;
  genre: string | null;
  year: number | null;
  trackNumber: number | null;
  discNumber: number | null;
  duration: number | null;
  artwork: string | null;
  addedAt: number;
  isFavorite: boolean;
}

export interface Album {
  id: number;
  title: string;
  artistId: number;
  artistName: string;
  year: number | null;
  genre: string | null;
  artwork: string | null;
  artworkFull: string | null;
  trackCount: number;
  duration: number;
  addedAt: number;
  isFavorite: boolean;
}

export interface Artist {
  id: number;
  name: string;
  artwork: string | null;
  albumCount: number;
  trackCount: number;
  isFavorite: boolean;
}

export interface AlbumDetail {
  album: Album;
  tracks: Track[];
  moreByArtist: Album[];
}

export interface ArtistDetail {
  artist: Artist;
  albums: Album[];
  topTracks: Track[];
}

export interface Genre {
  name: string;
  trackCount: number;
  artwork: string | null;
}

export interface Movie {
  id: number;
  mediaId: number;
  title: string;
  year: number | null;
  genres: string | null;
  overview: string | null;
  duration: number | null;
  poster: string | null;
  posterFull: string | null;
  backdrop: string | null;
  width: number | null;
  height: number | null;
  videoCodec: string | null;
  audioCodec: string | null;
  container: string | null;
  addedAt: number;
  position: number | null;
  completed: boolean | null;
  lastPlayedAt: number | null;
  isFavorite: boolean;
}

export interface StreamInfo {
  index: number;
  codec: string | null;
  language: string | null;
  title: string | null;
  channels: number | null;
  default: boolean;
}

export interface MovieDetail {
  movie: Movie;
  path: string;
  fileSize: number;
  audioStreams: StreamInfo[];
  subtitleStreams: StreamInfo[];
  moreLikeThis: Movie[];
}

export interface Show {
  id: number;
  title: string;
  year: number | null;
  overview: string | null;
  genres: string | null;
  poster: string | null;
  posterFull: string | null;
  backdrop: string | null;
  seasonCount: number;
  episodeCount: number;
  watchedCount: number;
  addedAt: number;
  isFavorite: boolean;
}

export interface Season {
  id: number;
  showId: number;
  seasonNumber: number;
  poster: string | null;
  episodeCount: number;
}

export interface Episode {
  id: number;
  mediaId: number;
  showId: number;
  showTitle: string;
  seasonNumber: number;
  episodeNumber: number;
  title: string;
  overview: string | null;
  duration: number | null;
  thumb: string | null;
  position: number | null;
  completed: boolean | null;
}

export interface ShowDetail {
  show: Show;
  seasons: Season[];
  episodes: Episode[];
  nextUp: Episode | null;
}

export interface PlayableItem {
  mediaId: number;
  kind: MediaKind;
  path: string;
  title: string;
  subtitle: string | null;
  artwork: string | null;
  backdrop: string | null;
  duration: number | null;
  trackId: number | null;
  albumId: number | null;
  albumTitle: string | null;
  artistId: number | null;
  movieId: number | null;
  episodeId: number | null;
  showId: number | null;
  position: number | null;
  completed: boolean | null;
  lastPlayedAt: number | null;
  missing: boolean;
  isFavorite: boolean;
}

export interface ContinueItem extends PlayableItem {
  reason: "resume" | "next";
}

export interface RecentItem {
  kind: "album" | "movie" | "tv_show";
  id: number;
  title: string;
  subtitle: string | null;
  artwork: string | null;
  addedAt: number;
}

export interface Playlist {
  id: number;
  name: string;
  description: string | null;
  artwork: string | null;
  createdAt: number;
  updatedAt: number;
  itemCount: number;
  duration: number;
  coverArtworks: string[];
}

export interface PlaylistEntry {
  itemId: number;
  position: number;
  addedAt: number;
  media: PlayableItem;
}

export interface PlaylistDetail {
  playlist: Playlist;
  entries: PlaylistEntry[];
}

export interface LibraryStats {
  tracks: number;
  albums: number;
  artists: number;
  movies: number;
  shows: number;
  episodes: number;
}

export interface HomeFeed {
  continueWatching: ContinueItem[];
  recentlyAdded: RecentItem[];
  recentlyPlayed: PlayableItem[];
  playlists: Playlist[];
  albums: Album[];
  movies: Movie[];
  shows: Show[];
  stats: LibraryStats;
}

export interface FavoritesOverview {
  tracks: Track[];
  albums: Album[];
  artists: Artist[];
  movies: Movie[];
  shows: Show[];
}

export interface SearchResults {
  query: string;
  tracks: Track[];
  albums: Album[];
  artists: Artist[];
  movies: Movie[];
  shows: Show[];
  playlists: Playlist[];
}

export interface ScanProgress {
  folderId: number | null;
  folderPath: string | null;
  phase: "discovering" | "indexing" | "artwork" | "idle";
  processed: number;
  total: number;
  currentPath: string | null;
  added: number;
  updated: number;
  removed: number;
  errors: number;
  active: boolean;
}

export interface TrackInfo {
  id: number;
  kind: "audio" | "sub";
  title: string | null;
  language: string | null;
  codec: string | null;
  selected: boolean;
  external: boolean;
}

export interface PlayerState {
  engine: EngineKind;
  item: PlayableItem | null;
  position: number;
  duration: number | null;
  paused: boolean;
  buffering: boolean;
  volume: number;
  muted: boolean;
  speed: number;
  fullscreen: boolean;
  audioTracks: TrackInfo[];
  subtitleTracks: TrackInfo[];
  active: boolean;
}

export interface PlayerEnded {
  engine: EngineKind;
  mediaId: number;
  reason: "eof" | "error" | "closed" | "stopped";
  nextMediaId: number | null;
  message: string | null;
}

export interface ToolStatus {
  mpv: string | null;
  ffmpeg: string | null;
  ffprobe: string | null;
}

export interface AppStatus {
  version: string;
  onboardingCompleted: boolean;
  dataDir: string;
  tools: ToolStatus;
  engines: { audio: EngineName; video: EngineName };
  folderCount: number;
  stats: LibraryStats;
}

export interface SubtitleFile {
  label: string;
  language: string | null;
  vtt: string;
}

export interface ProblemFile {
  mediaId: number;
  path: string;
  kind: MediaKind;
  error: string | null;
  missing: boolean;
}

export interface Notice {
  level: "info" | "warning" | "error";
  title: string;
  message: string;
}

export type PlayerCommand =
  | { type: "pause" }
  | { type: "resume" }
  | { type: "togglePause" }
  | { type: "seek"; position: number }
  | { type: "seekRelative"; offset: number }
  | { type: "setVolume"; volume: number }
  | { type: "setMuted"; muted: boolean }
  | { type: "setSpeed"; speed: number }
  | { type: "setAudioTrack"; id: number }
  | { type: "setSubtitleTrack"; id: number | null }
  | { type: "setFullscreen"; fullscreen: boolean }
  | { type: "stop" };

export type Settings = Record<string, string | number | boolean>;
