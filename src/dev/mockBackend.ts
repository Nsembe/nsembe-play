// DEVELOPMENT ONLY. Lets the UI run in a regular browser (`npm run dev`, open
// http://localhost:1420) without the Rust backend, using demo data. It is loaded
// only when `import.meta.env.DEV` is true and Tauri is absent, so it never ships
// in production builds and never appears in the desktop app.

/* eslint-disable @typescript-eslint/no-explicit-any */
import type { Album, Artist, Episode, Movie, PlayableItem, Playlist, Show, Track } from "@/types";

const now = Math.floor(Date.now() / 1000);

function poster(title: string, hue: number, w = 400, h = 600): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">
<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${hue},55%,38%)"/><stop offset="1" stop-color="hsl(${(hue + 60) % 360},45%,10%)"/></linearGradient>
<radialGradient id="r" cx="0.7" cy="0.25" r="0.7"><stop offset="0" stop-color="hsl(${(hue + 30) % 360},80%,70%)" stop-opacity="0.55"/><stop offset="1" stop-color="#000" stop-opacity="0"/></radialGradient></defs>
<rect width="100%" height="100%" fill="url(#g)"/><rect width="100%" height="100%" fill="url(#r)"/>
<text x="50%" y="${h - 60}" text-anchor="middle" font-family="Segoe UI, sans-serif" font-weight="800" font-size="${w / 11}" fill="white" fill-opacity="0.92">${title.replace(/&/g, "&amp;")}</text></svg>`;
  return "data:image/svg+xml;charset=utf-8," + encodeURIComponent(svg);
}

const movieSeed: [string, number, number, string, string][] = [
  ["Interstellar", 2014, 10140, "Science Fiction,Adventure,Drama", "A team of explorers travels through a wormhole in space in an attempt to ensure humanity's survival."],
  ["Arrival", 2016, 6960, "Science Fiction,Drama", "A linguist works with the military to communicate with alien lifeforms after twelve mysterious spacecraft appear around the world."],
  ["Inception", 2010, 8880, "Science Fiction,Action", "A thief who steals corporate secrets through dream-sharing technology is given the inverse task of planting an idea."],
  ["Blade Runner 2049", 2017, 9840, "Science Fiction,Drama", "A young blade runner's discovery of a long-buried secret leads him to track down a former blade runner."],
  ["Dune", 2021, 9300, "Science Fiction,Adventure", "A noble family becomes embroiled in a war for control over the galaxy's most valuable asset."],
  ["Whiplash", 2014, 6420, "Drama,Music", "A promising young drummer enrolls at a cut-throat music conservatory."],
  ["Spirited Away", 2001, 7500, "Animation,Fantasy", "A girl wanders into a world ruled by gods, witches and spirits."],
  ["Mad Max: Fury Road", 2015, 7200, "Action,Adventure", "In a post-apocalyptic wasteland, a woman rebels against a tyrannical ruler."],
  ["Parasite", 2019, 7920, "Thriller,Drama", "Greed and class discrimination threaten a newly formed symbiotic relationship."],
  ["Her", 2013, 7560, "Drama,Romance", "A lonely writer develops an unlikely relationship with an operating system."],
  ["Moonlight", 2016, 6660, "Drama", "A young man grapples with his identity and sexuality while growing up in Miami."],
  ["The Grand Budapest Hotel", 2014, 5940, "Comedy,Drama", "A legendary concierge and his protégé become embroiled in the theft of a priceless painting."],
];

const movies: Movie[] = movieSeed.map(([title, year, duration, genres, overview], i) => ({
  id: i + 1,
  mediaId: 100 + i,
  title,
  year,
  genres,
  overview,
  duration,
  poster: poster(title, (i * 47) % 360),
  posterFull: poster(title, (i * 47) % 360),
  backdrop: poster("", (i * 47 + 20) % 360, 1280, 720),
  width: 1920,
  height: 1080,
  videoCodec: "h264",
  audioCodec: "aac",
  container: "mp4",
  addedAt: now - i * 86400,
  position: i === 0 ? 4320 : i === 2 ? 2100 : null,
  completed: i === 5 ? true : null,
  lastPlayedAt: i === 0 ? now - 3600 : i === 2 ? now - 86400 : null,
  isFavorite: i === 1,
}));

const showSeed: [string, number, number[]][] = [
  ["Northern Lights", 2019, [8, 8]],
  ["The Long Road", 2021, [10]],
  ["Coastline", 2016, [6, 6, 6]],
];
const shows: Show[] = [];
const episodes: Episode[] = [];
showSeed.forEach(([title, year, seasons], si) => {
  const showId = si + 1;
  seasons.forEach((count, sIdx) => {
    for (let e = 1; e <= count; e++) {
      const id = episodes.length + 1;
      const done = si === 0 && sIdx === 0 && e < 3;
      episodes.push({
        id,
        mediaId: 300 + id,
        showId,
        showTitle: title,
        seasonNumber: sIdx + 1,
        episodeNumber: e,
        title: ["Pilot", "Arrival", "The Crossing", "Undertow", "Night Shift", "Echoes", "Lantern", "Homecoming", "Ashes", "Daybreak"][e - 1] ?? `Episode ${e}`,
        overview: "A quiet moment turns into a turning point for everyone involved.",
        duration: 2700,
        thumb: poster("", (si * 80 + e * 13) % 360, 640, 360),
        position: si === 0 && sIdx === 0 && e === 3 ? 900 : done ? 2700 : null,
        completed: done ? true : null,
      });
    }
  });
  shows.push({
    id: showId,
    title,
    year,
    overview: "An acclaimed drama following a group of strangers whose lives intersect in unexpected ways.",
    genres: "Drama,Mystery",
    poster: poster(title, (si * 80 + 200) % 360),
    posterFull: poster(title, (si * 80 + 200) % 360),
    backdrop: poster("", (si * 80 + 220) % 360, 1280, 720),
    seasonCount: seasons.length,
    episodeCount: seasons.reduce((a, b) => a + b, 0),
    watchedCount: si === 0 ? 2 : 0,
    addedAt: now - si * 3 * 86400,
    isFavorite: false,
  });
});

const albumSeed: [string, string, number, string, string[]][] = [
  ["Midnight Atlas", "Luna Vale", 2022, "Electronic", ["Signal", "Afterglow", "Nightswim", "Atlas", "Halflight", "Northbound"]],
  ["Copper Sky", "The Wanderers", 2019, "Indie", ["Copper Sky", "Paper Boats", "Hollow", "Kites", "Distant Shore"]],
  ["Lusaka Nights", "Mwila Banda", 2023, "Afrobeat", ["Kalindula", "Zambezi", "Cairo Road", "Sunset Boulevard", "Mukwai"]],
  ["Blue Hours", "June Okafor", 2021, "Jazz", ["Blue Hour", "Slow Tide", "Velvet", "Soft Machine"]],
  ["Static Bloom", "Glasshouse", 2020, "Alternative", ["Static", "Bloom", "Neon Fields", "Fracture", "Bloom (Reprise)"]],
  ["Golden Hour", "Luna Vale", 2019, "Electronic", ["Golden", "Mirage", "Overcast"]],
];
const artistNames = [...new Set(albumSeed.map((a) => a[1]))];
const artists: Artist[] = artistNames.map((name, i) => ({
  id: i + 1,
  name,
  artwork: poster("", (i * 61 + 30) % 360, 400, 400),
  albumCount: albumSeed.filter((a) => a[1] === name).length,
  trackCount: albumSeed.filter((a) => a[1] === name).reduce((n, a) => n + a[4].length, 0),
  isFavorite: i === 0,
}));
const albums: Album[] = [];
const tracks: Track[] = [];
albumSeed.forEach(([title, artist, year, genre, songs], ai) => {
  const artistId = artistNames.indexOf(artist) + 1;
  const art = poster(title, (ai * 53 + 120) % 360, 500, 500);
  songs.forEach((song, ti) => {
    const id = tracks.length + 1;
    tracks.push({
      id,
      mediaId: 1000 + id,
      title: song,
      artistId,
      artistName: artist,
      albumId: ai + 1,
      albumTitle: title,
      genre,
      year,
      trackNumber: ti + 1,
      discNumber: 1,
      duration: 160 + ((id * 37) % 140),
      artwork: art,
      addedAt: now - ai * 86400,
      isFavorite: id % 7 === 0,
    });
  });
  albums.push({
    id: ai + 1,
    title,
    artistId,
    artistName: artist,
    year,
    genre,
    artwork: art,
    artworkFull: art,
    trackCount: songs.length,
    duration: tracks.filter((t) => t.albumId === ai + 1).reduce((s, t) => s + (t.duration ?? 0), 0),
    addedAt: now - ai * 86400,
    isFavorite: ai === 2,
  });
});

function trackPlayable(t: Track): PlayableItem {
  return {
    mediaId: t.mediaId, kind: "track", path: "", title: t.title, subtitle: t.artistName, artwork: t.artwork, backdrop: t.artwork,
    duration: t.duration, trackId: t.id, albumId: t.albumId, albumTitle: t.albumTitle, artistId: t.artistId, movieId: null,
    episodeId: null, showId: null, position: null, completed: null, lastPlayedAt: now - 7200, missing: false, isFavorite: t.isFavorite,
  };
}
function moviePlayable(m: Movie): PlayableItem {
  return {
    mediaId: m.mediaId, kind: "movie", path: "", title: m.title, subtitle: String(m.year), artwork: m.poster, backdrop: m.backdrop,
    duration: m.duration, trackId: null, albumId: null, albumTitle: null, artistId: null, movieId: m.id, episodeId: null, showId: null,
    position: m.position, completed: m.completed, lastPlayedAt: m.lastPlayedAt, missing: false, isFavorite: m.isFavorite,
  };
}
function episodePlayable(e: Episode): PlayableItem {
  const code = `S${String(e.seasonNumber).padStart(2, "0")}E${String(e.episodeNumber).padStart(2, "0")}`;
  return {
    mediaId: e.mediaId, kind: "episode", path: "", title: e.title, subtitle: `${e.showTitle} · ${code}`, artwork: e.thumb,
    backdrop: shows[e.showId - 1]?.backdrop ?? null, duration: e.duration, trackId: null, albumId: null, albumTitle: null, artistId: null,
    movieId: null, episodeId: e.id, showId: e.showId, position: e.position, completed: e.completed, lastPlayedAt: now - 1800,
    missing: false, isFavorite: false,
  };
}
function playable(mediaId: number): PlayableItem | undefined {
  const t = tracks.find((x) => x.mediaId === mediaId);
  if (t) return trackPlayable(t);
  const m = movies.find((x) => x.mediaId === mediaId);
  if (m) return moviePlayable(m);
  const e = episodes.find((x) => x.mediaId === mediaId);
  return e ? episodePlayable(e) : undefined;
}

const playlists: Playlist[] = [
  { id: 1, name: "Late Night Drive", description: "Smooth electronics for the open road", artwork: null, createdAt: now - 864000, updatedAt: now - 3600, itemCount: 8, duration: 1800, coverArtworks: albums.slice(0, 4).map((a) => a.artwork!) },
  { id: 2, name: "Sunday Movies", description: null, artwork: null, createdAt: now - 400000, updatedAt: now - 7200, itemCount: 3, duration: 26000, coverArtworks: [movies[0].poster!] },
];
const playlistItems: Record<number, number[]> = {
  1: tracks.slice(0, 8).map((t) => t.mediaId),
  2: movies.slice(0, 3).map((m) => m.mediaId),
};

const page = <T,>(items: T[], a: any) => {
  const offset = a?.offset ?? 0;
  const limit = a?.limit ?? 60;
  return { items: items.slice(offset, offset + limit), total: items.length, offset, limit };
};

const settings: Record<string, any> = {
  "app.onboardingCompleted": true, "player.audioEngine": "auto", "player.videoEngine": "auto", "player.autoplayNextEpisode": true,
  "player.videoFullscreen": false, "player.volume": 80, "player.audioLanguage": "", "player.subtitleLanguage": "",
  "tools.mpvPath": "", "tools.ffmpegPath": "", "tools.ffprobePath": "", "library.watchFolders": true, "library.videoThumbnails": true,
};

const stats = { tracks: tracks.length, albums: albums.length, artists: artists.length, movies: movies.length, shows: shows.length, episodes: episodes.length };

function showDetail(id: number) {
  const show = shows[id - 1];
  const eps = episodes.filter((e) => e.showId === id);
  const seasons = [...new Set(eps.map((e) => e.seasonNumber))].map((n) => ({
    id: id * 100 + n, showId: id, seasonNumber: n, poster: show.poster, episodeCount: eps.filter((e) => e.seasonNumber === n).length,
  }));
  return { show, seasons, episodes: eps, nextUp: eps.find((e) => !e.completed) ?? eps[0] };
}

const handlers: Record<string, (a: any) => any> = {
  app_status: () => ({
    version: "0.1.0-dev", onboardingCompleted: !new URLSearchParams(location.search).has("welcome"), dataDir: "C:\\Users\\Demo\\AppData\\Local\\com.nsembe.play",
    tools: { mpv: null, ffmpeg: null, ffprobe: null }, engines: { audio: "builtin", video: "builtin" }, folderCount: 3, stats,
  }),
  settings_get: () => settings,
  settings_set: (a) => ((settings[a.key] = a.value), settings),
  library_folders: () => [
    { id: 1, path: "D:\\Media\\Music", kind: "music", enabled: true, addedAt: now - 864000, lastScannedAt: now - 600, lastError: null, itemCount: tracks.length, missingCount: 0, errorCount: 1, accessible: true },
    { id: 2, path: "D:\\Media\\Movies", kind: "movies", enabled: true, addedAt: now - 864000, lastScannedAt: now - 600, lastError: null, itemCount: movies.length, missingCount: 1, errorCount: 0, accessible: true },
    { id: 3, path: "E:\\TV", kind: "tv", enabled: true, addedAt: now - 864000, lastScannedAt: null, lastError: "E:\\TV is not accessible. Is the drive connected?", itemCount: episodes.length, missingCount: 0, errorCount: 0, accessible: false },
  ],
  library_scan_status: () => ({ folderId: 1, folderPath: "D:\\Media\\Music", phase: "indexing", processed: 412, total: 1280, currentPath: "D:\\Media\\Music\\Luna Vale\\Midnight Atlas\\03 - Nightswim.flac", added: 400, updated: 12, removed: 0, errors: 0, active: true }),
  library_problem_files: () => [],
  home_feed: () => ({
    continueWatching: [
      { ...moviePlayable(movies[0]), reason: "resume" },
      { ...episodePlayable(episodes[2]), reason: "resume" },
      { ...moviePlayable(movies[2]), reason: "resume" },
    ],
    recentlyAdded: [
      ...albums.slice(0, 3).map((a) => ({ kind: "album", id: a.id, title: a.title, subtitle: a.artistName, artwork: a.artwork, addedAt: a.addedAt })),
      ...movies.slice(3, 7).map((m) => ({ kind: "movie", id: m.id, title: m.title, subtitle: String(m.year), artwork: m.poster, addedAt: m.addedAt })),
      ...shows.slice(0, 2).map((s) => ({ kind: "tv_show", id: s.id, title: s.title, subtitle: `${s.episodeCount} episodes`, artwork: s.poster, addedAt: s.addedAt })),
    ],
    recentlyPlayed: tracks.slice(5, 12).map(trackPlayable),
    playlists,
    albums,
    movies: movies.slice(0, 10),
    shows,
    stats,
  }),
  recently_added: (a) => page(albums.map((x) => ({ kind: "album", id: x.id, title: x.title, subtitle: x.artistName, artwork: x.artwork, addedAt: x.addedAt })), a),
  recently_played: (a) => page([moviePlayable(movies[0]), ...tracks.slice(0, 10).map(trackPlayable), episodePlayable(episodes[2])], a),
  continue_watching: () => [],
  movies_list: (a) => {
    let list = movies;
    if (a?.genre) list = list.filter((m) => m.genres?.split(",").includes(a.genre));
    if (a?.sort === "year") list = [...list].sort((x, y) => (y.year ?? 0) - (x.year ?? 0));
    else if (a?.sort !== "added") list = [...list].sort((x, y) => x.title.localeCompare(y.title));
    return page(list, a);
  },
  movie_genres: () => [...new Set(movies.flatMap((m) => m.genres!.split(",")))].sort(),
  movie_detail: (a) => {
    const movie = movies.find((m) => m.id === a.id)!;
    return {
      movie, path: "D:\\Media\\Movies\\" + movie.title + ".mp4", fileSize: 4_812_000_000,
      audioStreams: [{ index: 1, codec: "eac3", language: "eng", title: null, channels: 6, default: true }, { index: 2, codec: "aac", language: "fre", title: null, channels: 2, default: false }],
      subtitleStreams: [{ index: 3, codec: "subrip", language: "eng", title: null, channels: null, default: false }],
      moreLikeThis: movies.filter((m) => m.id !== movie.id && m.genres?.split(",").some((g) => movie.genres?.includes(g))).slice(0, 8),
    };
  },
  shows_list: (a) => page(shows, a),
  show_detail: (a) => showDetail(a.id),
  tracks_list: (a) => page(tracks, a),
  genre_tracks: (a) => page(tracks.filter((t) => t.genre === a.genre), a),
  genres_list: () => [...new Set(tracks.map((t) => t.genre!))].map((g) => ({ name: g, trackCount: tracks.filter((t) => t.genre === g).length, artwork: tracks.find((t) => t.genre === g)?.artwork ?? null })),
  albums_list: (a) => page(albums, a),
  album_detail: (a) => {
    const album = albums.find((x) => x.id === a.id)!;
    return { album, tracks: tracks.filter((t) => t.albumId === album.id), moreByArtist: albums.filter((x) => x.artistId === album.artistId && x.id !== album.id) };
  },
  artists_list: (a) => page(artists, a),
  artist_detail: (a) => {
    const artist = artists.find((x) => x.id === a.id)!;
    return { artist, albums: albums.filter((x) => x.artistId === artist.id), topTracks: tracks.filter((t) => t.artistId === artist.id) };
  },
  favorites_overview: () => ({
    tracks: tracks.filter((t) => t.isFavorite), albums: albums.filter((x) => x.isFavorite), artists: artists.filter((x) => x.isFavorite),
    movies: movies.filter((m) => m.isFavorite), shows: [],
  }),
  favorite_toggle: () => true,
  search: (a) => {
    const q = String(a.query).toLowerCase();
    const has = (s: string | null | undefined) => !!s && s.toLowerCase().includes(q.slice(0, 4));
    return {
      query: a.query, tracks: tracks.filter((t) => has(t.title) || has(t.artistName)).slice(0, 8), albums: albums.filter((x) => has(x.title) || has(x.artistName)),
      artists: artists.filter((x) => has(x.name)), movies: movies.filter((m) => has(m.title)), shows: shows.filter((s) => has(s.title)), playlists: playlists.filter((p) => has(p.name)),
    };
  },
  playlists_list: () => playlists,
  playlist_detail: (a) => ({
    playlist: playlists.find((p) => p.id === a.id)!,
    entries: (playlistItems[a.id] ?? []).map((m, i) => ({ itemId: a.id * 1000 + i, position: i, addedAt: now - i * 5000, media: playable(m)! })),
  }),
  media_playables: (a) => (a.mediaIds as number[]).map(playable).filter(Boolean),
  music_shuffle: () => tracks.map(trackPlayable),
  playback_resolve: (a) => playable(a.mediaId),
  media_subtitles: () => [],
  player_adjacent_episode: () => null,
  player_state: (a) => ({ engine: a.engineKind, item: null, position: 0, duration: null, paused: true, buffering: false, volume: 80, muted: false, speed: 1, fullscreen: false, audioTracks: [], subtitleTracks: [], active: false }),
};

export function installMockBackend() {
  const params = new URLSearchParams(location.search);
  // `?player` seeds a queue so the player bar can be designed without audio files.
  if (params.has("player")) {
    const queue = tracks.slice(0, 12).map(trackPlayable);
    localStorage.setItem("nsembe-player", JSON.stringify({ state: { queue, index: 2, shuffle: false, repeat: "all", volume: 70 }, version: 0 }));
    void import("@/stores/playerStore").then((m) => m.usePlayerStore.persist.rehydrate());
  }
  let callbackId = 0;
  (window as any).__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
    transformCallback: () => ++callbackId,
    unregisterCallback: () => {},
    convertFileSrc: (p: string) => p,
    invoke: async (cmd: string, args: any) => {
      await new Promise((r) => setTimeout(r, 60));
      if (cmd.startsWith("plugin:")) return cmd.endsWith("|listen") ? ++callbackId : null;
      const h = handlers[cmd];
      if (!h) {
        console.info(`[mock] ${cmd}`, args);
        return null;
      }
      return structuredClone(h(args ?? {}));
    },
  };
  (window as any).__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
  console.info("%cNsembe Play: running with the development mock backend", "color:#78F760");
}
