# Nsembe Play

**Your media. Your library. Your device.**

Nsembe Play is a local-first desktop media app for Windows. It turns your own movie, TV and music folders into a streaming-style library (Netflix-style browsing for video, Spotify-style music), and plays everything offline. There is no account, no cloud and no telemetry. Files are indexed where they are and are never copied or uploaded.

Built with **Tauri 2 + Rust** (engine) and **React + TypeScript + Tailwind** (UI), with **SQLite/FTS5** for the library and **mpv / FFmpeg** for playback and media inspection.

---

## Features

| Area | What works |
| --- | --- |
| Library | Add music / movie / TV folders; background incremental scans; live folder watching (new, deleted, renamed and moved files); missing-file tracking; duplicate/nested folder protection; files that couldn't be read are shown in Settings |
| Metadata | Embedded tags (ID3, Vorbis, MP4, FLAC…) via `lofty`; FFprobe for duration, resolution, codecs, audio and subtitle streams; Kodi-style `.nfo` files; robust filename parsing (`Movie (2010).mkv`, scene names, `S01E02`, `1x02`, `Season 01/` folders) |
| Artwork | Embedded album covers, `cover.jpg`/`folder.jpg`, `poster.jpg`/`fanart.jpg`, season posters, FFmpeg frame grabs for movies/episodes, generated in the background into a local cache |
| Browsing | Home (hero, Continue Watching, Recently Added/Played, Playlists, Music, Movies, TV), Movies (sort + genre filter), Movie detail with "More Like This", TV Shows → Seasons → Episodes, Music (Songs / Artists / Albums / Genres), Album and Artist pages |
| Playback | mpv engine (audio in the background, video in its own window placed over the app) with full remote control (seek, volume, speed, audio and subtitle tracks, fullscreen, next/previous episode); built-in webview player as a fallback; resume position, 90% completion rule, next-episode autoplay |
| Music player | Persistent bottom player, queue with drag-and-drop, shuffle, repeat all/one, volume, expanded Now Playing view; the queue survives restarts |
| Organisation | Playlists (create, rename, delete, add/remove, drag-and-drop reorder, mixed songs and videos, play/shuffle); Favorites for songs, albums, artists, movies and shows; Recently Played with history clearing |
| Search | Global fuzzy search (FTS5 trigram index plus re-ranking) grouped into Songs, Artists, Albums, Movies, TV Shows and Playlists; tolerates typos ("intersteller", "pink floid") |
| UX | First-run onboarding with live indexing progress, polished empty states, a collapsible sidebar, keyboard shortcuts and friendly error messages |

### Keyboard shortcuts

| Action | Keys |
| --- | --- |
| Play / pause | `Space` |
| Search | `Ctrl+K` or `/` |
| Next / previous song | `Ctrl+→` / `Ctrl+←` |
| Volume | `Ctrl+↑` / `Ctrl+↓` |
| Mute | `M` |
| Settings | `Ctrl+,` |
| Video: seek, fullscreen, next/previous episode | `←/→`, `F`, `N`/`P` |

---

## Getting started (development)

### Prerequisites (Windows)

1. **Rust** 1.88+ with the MSVC toolchain: <https://rustup.rs>
2. **Visual Studio 2022 Build Tools** with "Desktop development with C++" (needed by Rust/MSVC)
3. **Node.js** 20+ (22 recommended)
4. **WebView2**, which is preinstalled on Windows 10/11

### Install and run

```powershell
npm install
npm run fetch-tools      # optional but recommended: downloads mpv, ffmpeg, ffprobe into src-tauri/resources/bin
npm run app:dev          # starts Vite + the Tauri app with hot reload
```

On first launch you'll see the onboarding screen. Add your folders, for example:

```
Music:     D:\Media\Music
Movies:    D:\Media\Movies
TV Shows:  D:\Media\TV        (Show Name\Season 01\S01E01.mkv)
```

**UI-only development:** `npm run dev` and open <http://localhost:1420> in a normal browser. Outside Tauri, the UI automatically uses a development-only mock backend with demo data (`src/dev/mockBackend.ts`). Add `?player` to the URL to seed a music queue, or `?welcome` to see onboarding. The mock is never included in production builds.

### Media tools (mpv, FFmpeg, FFprobe)

All three are **optional**. The app finds them, in this order:

1. A path set in **Settings → Media tools**
2. The bundled `bin` folder next to the app (`src-tauri/resources/bin` during development)
3. `PATH`, then common install locations (`Program Files`, Scoop, Chocolatey, WinGet)

| Missing tool | Effect |
| --- | --- |
| mpv | Music and video use the built-in webview player (MP3/FLAC/AAC/OGG/WAV, MP4/WebM). Formats such as MKV/HEVC show a clear "install mpv" message |
| ffprobe | Video durations are filled in on first playback; resolution and codec details are not shown |
| ffmpeg | No video thumbnails or backdrops; local `poster.jpg`/`fanart.jpg` files are still used |

`npm run fetch-tools` (`scripts/fetch-tools.ps1`) downloads the FFmpeg "essentials" build from gyan.dev and the latest mpv build from `shinchiro/mpv-winbuild-cmake`. **Licensing:** these are (L)GPL third-party binaries. Review their licenses before shipping them inside your installer.

---

## Building for Windows

```powershell
npm run fetch-tools      # if you want mpv/FFmpeg bundled into the installer
npm run app:build
```

Output:

```
src-tauri/target/release/nsembe-play.exe
src-tauri/target/release/bundle/nsis/Nsembe Play_0.1.0_x64-setup.exe   (per-user installer)
src-tauri/target/release/bundle/msi/Nsembe Play_0.1.0_x64_en-US.msi
```

Everything in `src-tauri/resources/bin` is bundled and found automatically at runtime. For public distribution you should also configure code signing (`bundle.windows.certificateThumbprint` in `src-tauri/tauri.conf.json`).

### Tests

```powershell
npm run test:rust        # cargo test --workspace
npm run typecheck        # TypeScript
```

The Rust suite covers filename parsing, `.nfo` and FFprobe parsing, fuzzy search ranking, full library scans of a temporary media tree (incremental rescans, missing and moved files, watcher pickup), continue-watching and next-episode rules, playlists and favorites. `core/tests/media_tools.rs` also runs real FFmpeg/FFprobe/mpv (generating media, scanning it and playing it over mpv IPC) when the tools are present, and skips otherwise.

---

## Architecture

```
React UI (src/)                       presentation, navigation, UI state (Zustand), React Query cache
   │  typed invoke() wrappers (src/lib/api.ts) + events (src/lib/events.ts)
   ▼
Tauri shell (src-tauri/src/)          thin command layer, asset-protocol scoping, event forwarding
   ▼
nsembe-core (src-tauri/core/)         UI-agnostic media engine (no Tauri dependency)
   ├─ database/     SQLite (SQLx), migrations, every SQL query, FTS5 search
   ├─ scanner/      incremental indexing pipeline + filename parsers
   ├─ filesystem/   discovery, fingerprints, debounced folder watcher (notify)
   ├─ metadata/     lofty (audio tags), ffprobe, .nfo, subtitles, provider trait (TMDB/MusicBrainz later)
   ├─ thumbnails/   resumable background artwork pass (image crate + ffmpeg)
   ├─ player/       mpv processes over JSON IPC, progress persistence, next-episode logic
   ├─ library/      folder management and watcher → scanner routing
   ├─ settings/     typed settings
   └─ engine.rs     MediaEngine facade + EventSink trait for any client
   ▼
SQLite (%LOCALAPPDATA%\com.nsembe.play\library.db)   +   your media files (read in place)
```

Key decisions:

- **The media engine is a separate crate** (`nsembe-core`) with no Tauri dependency. Another client, such as a CLI, a home-server daemon or a mobile app, can embed `MediaEngine` and receive events through `EventSink`.
- **Rust owns all file access.** There is no Tauri filesystem plugin. React gets a folder path from the native picker and hands it to Rust, which validates and indexes it. The webview can only read the artwork cache and the library folders you added, through the scoped asset protocol, which the built-in player needs. See `src-tauri/capabilities/default.json` for the permission allowlist.
- **mpv is controlled over its JSON IPC protocol** (named pipe) rather than linked as libmpv. This keeps the build simple, works with any mpv build, and isolates crashes from corrupt media in a separate process. The video plays in mpv's own window, sized and positioned over the app window, while the app shows a synced "remote" screen.
- **Scanning never blocks the UI.** A single background worker serializes database writes. Metadata is extracted in parallel, unchanged files (same size and mtime) are skipped, and artwork is produced in a later, resumable pass. Files still being copied are retried until their size settles.
- **Moves and renames keep history.** Watcher rename events update paths in place. Moves the watcher didn't see are matched by a content fingerprint (size plus a hash of the first and last 64 KiB), so playlists, favorites and resume positions survive.
- **Missing files are flagged, not deleted.** A disconnected drive won't wipe your library. Missing items are hidden from views and forgotten after 30 days, or immediately from Settings.
- **Large libraries stay fast.** Grids and song lists are virtualized and page in from SQLite, so React never holds the whole library. Thumbnails (360px) are used in lists and full-size artwork only on detail pages.

### Database

`src-tauri/core/migrations/0001_initial_schema.sql` defines `library_folders`, `media`, `artists`, `albums`, `tracks`, `movies`, `tv_shows`, `tv_seasons`, `tv_episodes`, `playlists`, `playlist_items`, `favorites`, `play_history`, `playback_progress` and `settings`. It uses foreign keys with cascades, indexes for every browse path, and a trigram FTS5 `search_index` kept in sync by triggers. Only paths and metadata are stored, never media files.

### Project structure

```
src/
  components/        shared UI (cards, shelves, artwork, virtual grid, shadcn-style primitives in ui/)
  features/          domain UI: library/, movies/, music/, player/, playlists/
  layouts/           app shell: sidebar, top bar, scroll context
  pages/             routed screens
  stores/            Zustand stores (player queue, UI, scan progress)
  hooks/ lib/ types/ helpers, typed API, event bridge, DTO types
  dev/               development-only mock backend
src-tauri/
  src/               Tauri app: commands/, events, error mapping
  core/              nsembe-core engine crate (+ migrations/, tests/)
  capabilities/      Tauri permission allowlist
  resources/bin/     optional bundled mpv/ffmpeg/ffprobe
scripts/fetch-tools.ps1
```

---

## Data and privacy

- Library database, artwork cache and logs: `%LOCALAPPDATA%\com.nsembe.play\`
- Logs: `%LOCALAPPDATA%\com.nsembe.play\logs\nsembe-play.log`
- Nothing leaves the device. No network access is required or attempted (mpv runs with `--ytdl=no`).

## Known limitations and next steps

- Video under mpv plays in mpv's own window positioned over the app, not embedded in the webview. Embedding through libmpv's render API is the next step for a fully in-window player.
- The built-in fallback player can only play formats WebView2 supports, and it can't switch embedded audio tracks or show embedded subtitles. Sidecar `.srt`/`.vtt` subtitles are supported.
- Search results can't be navigated with the keyboard yet.
- Online metadata (TMDB, MusicBrainz) is not implemented. The `MetadataProvider` trait in `core/src/metadata/provider.rs` is the extension point, and the core does not depend on it.
- Installers are unsigned.
