-- Nsembe Play initial schema.
-- Media files are never stored in the database: only paths and metadata.
-- Timestamps are unix epoch seconds.

CREATE TABLE library_folders (
    id              INTEGER PRIMARY KEY,
    path            TEXT    NOT NULL UNIQUE,
    kind            TEXT    NOT NULL CHECK (kind IN ('music', 'movies', 'tv')),
    enabled         INTEGER NOT NULL DEFAULT 1,
    added_at        INTEGER NOT NULL,
    last_scanned_at INTEGER,
    last_error      TEXT
);

CREATE TABLE media (
    id               INTEGER PRIMARY KEY,
    folder_id        INTEGER NOT NULL REFERENCES library_folders(id) ON DELETE CASCADE,
    path             TEXT    NOT NULL UNIQUE,
    kind             TEXT    NOT NULL CHECK (kind IN ('track', 'movie', 'episode')),
    file_size        INTEGER NOT NULL,
    file_mtime       INTEGER NOT NULL,
    fingerprint      TEXT,
    duration         REAL,
    container        TEXT,
    video_codec      TEXT,
    audio_codec      TEXT,
    width            INTEGER,
    height           INTEGER,
    bitrate          INTEGER,
    audio_streams    TEXT,          -- JSON array of stream descriptions
    subtitle_streams TEXT,          -- JSON array of stream descriptions
    thumb_path       TEXT,          -- generated frame/thumbnail (videos)
    artwork_checked  INTEGER NOT NULL DEFAULT 0,
    missing          INTEGER NOT NULL DEFAULT 0,
    missing_since    INTEGER,
    scan_error       TEXT,
    added_at         INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL
);
CREATE INDEX idx_media_folder ON media(folder_id);
CREATE INDEX idx_media_kind_added ON media(kind, missing, added_at DESC);
CREATE INDEX idx_media_fingerprint ON media(fingerprint) WHERE fingerprint IS NOT NULL;

CREATE TABLE artists (
    id           INTEGER PRIMARY KEY,
    name         TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    sort_name    TEXT,
    artwork_path TEXT,
    added_at     INTEGER NOT NULL
);

CREATE TABLE albums (
    id              INTEGER PRIMARY KEY,
    title           TEXT    NOT NULL COLLATE NOCASE,
    artist_id       INTEGER NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    year            INTEGER,
    genre           TEXT,
    artwork_path    TEXT,
    artwork_thumb   TEXT,
    artwork_checked INTEGER NOT NULL DEFAULT 0,
    added_at        INTEGER NOT NULL,
    UNIQUE (title, artist_id)
);
CREATE INDEX idx_albums_artist ON albums(artist_id);
CREATE INDEX idx_albums_added ON albums(added_at DESC);

CREATE TABLE tracks (
    id           INTEGER PRIMARY KEY,
    media_id     INTEGER NOT NULL UNIQUE REFERENCES media(id) ON DELETE CASCADE,
    title        TEXT    NOT NULL,
    artist_id    INTEGER REFERENCES artists(id) ON DELETE SET NULL,
    album_id     INTEGER REFERENCES albums(id) ON DELETE SET NULL,
    genre        TEXT,
    year         INTEGER,
    track_number INTEGER,
    disc_number  INTEGER
);
CREATE INDEX idx_tracks_album ON tracks(album_id, disc_number, track_number);
CREATE INDEX idx_tracks_artist ON tracks(artist_id);
CREATE INDEX idx_tracks_genre ON tracks(genre COLLATE NOCASE);
CREATE INDEX idx_tracks_title ON tracks(title COLLATE NOCASE);

CREATE TABLE movies (
    id              INTEGER PRIMARY KEY,
    media_id        INTEGER NOT NULL UNIQUE REFERENCES media(id) ON DELETE CASCADE,
    title           TEXT    NOT NULL,
    sort_title      TEXT    NOT NULL,
    year            INTEGER,
    genres          TEXT,           -- comma separated
    overview        TEXT,
    poster_path     TEXT,
    poster_thumb    TEXT,
    backdrop_path   TEXT,
    added_at        INTEGER NOT NULL
);
CREATE INDEX idx_movies_sort ON movies(sort_title COLLATE NOCASE);
CREATE INDEX idx_movies_added ON movies(added_at DESC);

CREATE TABLE tv_shows (
    id              INTEGER PRIMARY KEY,
    title           TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    folder_path     TEXT,
    year            INTEGER,
    overview        TEXT,
    genres          TEXT,
    poster_path     TEXT,
    poster_thumb    TEXT,
    backdrop_path   TEXT,
    artwork_checked INTEGER NOT NULL DEFAULT 0,
    added_at        INTEGER NOT NULL
);

CREATE TABLE tv_seasons (
    id            INTEGER PRIMARY KEY,
    show_id       INTEGER NOT NULL REFERENCES tv_shows(id) ON DELETE CASCADE,
    season_number INTEGER NOT NULL,
    folder_path   TEXT,
    poster_path   TEXT,
    UNIQUE (show_id, season_number)
);

CREATE TABLE tv_episodes (
    id             INTEGER PRIMARY KEY,
    media_id       INTEGER NOT NULL UNIQUE REFERENCES media(id) ON DELETE CASCADE,
    show_id        INTEGER NOT NULL REFERENCES tv_shows(id) ON DELETE CASCADE,
    season_id      INTEGER NOT NULL REFERENCES tv_seasons(id) ON DELETE CASCADE,
    season_number  INTEGER NOT NULL,
    episode_number INTEGER NOT NULL,
    title          TEXT    NOT NULL,
    overview       TEXT
);
CREATE INDEX idx_episodes_order ON tv_episodes(show_id, season_number, episode_number);
CREATE INDEX idx_episodes_season ON tv_episodes(season_id);

CREATE TABLE playlists (
    id           INTEGER PRIMARY KEY,
    name         TEXT    NOT NULL,
    description  TEXT,
    artwork_path TEXT,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
);

CREATE TABLE playlist_items (
    id          INTEGER PRIMARY KEY,
    playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    media_id    INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    added_at    INTEGER NOT NULL
);
CREATE INDEX idx_playlist_items_order ON playlist_items(playlist_id, position);
CREATE INDEX idx_playlist_items_media ON playlist_items(media_id);

-- Polymorphic favorites. Integrity is kept by the delete triggers below.
CREATE TABLE favorites (
    item_type  TEXT    NOT NULL CHECK (item_type IN ('track', 'album', 'artist', 'movie', 'tv_show')),
    item_id    INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (item_type, item_id)
);
CREATE INDEX idx_favorites_recent ON favorites(item_type, created_at DESC);

-- Append-only log of play sessions.
CREATE TABLE play_history (
    id        INTEGER PRIMARY KEY,
    media_id  INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    played_at INTEGER NOT NULL,
    position  REAL    NOT NULL DEFAULT 0,
    duration  REAL
);
CREATE INDEX idx_history_played ON play_history(played_at DESC);
CREATE INDEX idx_history_media ON play_history(media_id);

-- Latest resume state per media item (continue watching / listening).
CREATE TABLE playback_progress (
    media_id       INTEGER PRIMARY KEY REFERENCES media(id) ON DELETE CASCADE,
    position       REAL    NOT NULL DEFAULT 0,
    duration       REAL,
    completed      INTEGER NOT NULL DEFAULT 0,
    play_count     INTEGER NOT NULL DEFAULT 0,
    last_played_at INTEGER NOT NULL
);
CREATE INDEX idx_progress_recent ON playback_progress(last_played_at DESC);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- ---------------------------------------------------------------------------
-- Full-text search. A trigram tokenizer gives substring matching and, combined
-- with re-ranking in Rust, tolerance for small spelling mistakes.
-- rowid = ref_id * 8 + kind code so that rows can be updated by primary key:
--   1 track, 2 album, 3 artist, 4 movie, 5 tv_show, 6 playlist
-- ---------------------------------------------------------------------------
CREATE VIRTUAL TABLE search_index USING fts5(
    kind UNINDEXED,
    ref_id UNINDEXED,
    title,
    subtitle,
    tokenize = 'trigram'
);

-- tracks
CREATE TRIGGER trg_tracks_ai AFTER INSERT ON tracks BEGIN
    INSERT INTO search_index(rowid, kind, ref_id, title, subtitle)
    VALUES (new.id * 8 + 1, 'track', new.id, new.title,
            COALESCE((SELECT name FROM artists WHERE id = new.artist_id), '') || ' ' ||
            COALESCE((SELECT title FROM albums WHERE id = new.album_id), ''));
END;
CREATE TRIGGER trg_tracks_au AFTER UPDATE OF title, artist_id, album_id ON tracks BEGIN
    DELETE FROM search_index WHERE rowid = old.id * 8 + 1;
    INSERT INTO search_index(rowid, kind, ref_id, title, subtitle)
    VALUES (new.id * 8 + 1, 'track', new.id, new.title,
            COALESCE((SELECT name FROM artists WHERE id = new.artist_id), '') || ' ' ||
            COALESCE((SELECT title FROM albums WHERE id = new.album_id), ''));
END;
CREATE TRIGGER trg_tracks_ad AFTER DELETE ON tracks BEGIN
    DELETE FROM search_index WHERE rowid = old.id * 8 + 1;
    DELETE FROM favorites WHERE item_type = 'track' AND item_id = old.id;
END;

-- albums
CREATE TRIGGER trg_albums_ai AFTER INSERT ON albums BEGIN
    INSERT INTO search_index(rowid, kind, ref_id, title, subtitle)
    VALUES (new.id * 8 + 2, 'album', new.id, new.title,
            COALESCE((SELECT name FROM artists WHERE id = new.artist_id), ''));
END;
CREATE TRIGGER trg_albums_au AFTER UPDATE OF title, artist_id ON albums BEGIN
    DELETE FROM search_index WHERE rowid = old.id * 8 + 2;
    INSERT INTO search_index(rowid, kind, ref_id, title, subtitle)
    VALUES (new.id * 8 + 2, 'album', new.id, new.title,
            COALESCE((SELECT name FROM artists WHERE id = new.artist_id), ''));
END;
CREATE TRIGGER trg_albums_ad AFTER DELETE ON albums BEGIN
    DELETE FROM search_index WHERE rowid = old.id * 8 + 2;
    DELETE FROM favorites WHERE item_type = 'album' AND item_id = old.id;
END;

-- artists
CREATE TRIGGER trg_artists_ai AFTER INSERT ON artists BEGIN
    INSERT INTO search_index(rowid, kind, ref_id, title, subtitle)
    VALUES (new.id * 8 + 3, 'artist', new.id, new.name, '');
END;
CREATE TRIGGER trg_artists_au AFTER UPDATE OF name ON artists BEGIN
    DELETE FROM search_index WHERE rowid = old.id * 8 + 3;
    INSERT INTO search_index(rowid, kind, ref_id, title, subtitle)
    VALUES (new.id * 8 + 3, 'artist', new.id, new.name, '');
END;
CREATE TRIGGER trg_artists_ad AFTER DELETE ON artists BEGIN
    DELETE FROM search_index WHERE rowid = old.id * 8 + 3;
    DELETE FROM favorites WHERE item_type = 'artist' AND item_id = old.id;
END;

-- movies
CREATE TRIGGER trg_movies_ai AFTER INSERT ON movies BEGIN
    INSERT INTO search_index(rowid, kind, ref_id, title, subtitle)
    VALUES (new.id * 8 + 4, 'movie', new.id, new.title,
            COALESCE(CAST(new.year AS TEXT), '') || ' ' || COALESCE(new.genres, ''));
END;
CREATE TRIGGER trg_movies_au AFTER UPDATE OF title, year, genres ON movies BEGIN
    DELETE FROM search_index WHERE rowid = old.id * 8 + 4;
    INSERT INTO search_index(rowid, kind, ref_id, title, subtitle)
    VALUES (new.id * 8 + 4, 'movie', new.id, new.title,
            COALESCE(CAST(new.year AS TEXT), '') || ' ' || COALESCE(new.genres, ''));
END;
CREATE TRIGGER trg_movies_ad AFTER DELETE ON movies BEGIN
    DELETE FROM search_index WHERE rowid = old.id * 8 + 4;
    DELETE FROM favorites WHERE item_type = 'movie' AND item_id = old.id;
END;

-- tv shows
CREATE TRIGGER trg_shows_ai AFTER INSERT ON tv_shows BEGIN
    INSERT INTO search_index(rowid, kind, ref_id, title, subtitle)
    VALUES (new.id * 8 + 5, 'tv_show', new.id, new.title, COALESCE(new.genres, ''));
END;
CREATE TRIGGER trg_shows_au AFTER UPDATE OF title, genres ON tv_shows BEGIN
    DELETE FROM search_index WHERE rowid = old.id * 8 + 5;
    INSERT INTO search_index(rowid, kind, ref_id, title, subtitle)
    VALUES (new.id * 8 + 5, 'tv_show', new.id, new.title, COALESCE(new.genres, ''));
END;
CREATE TRIGGER trg_shows_ad AFTER DELETE ON tv_shows BEGIN
    DELETE FROM search_index WHERE rowid = old.id * 8 + 5;
    DELETE FROM favorites WHERE item_type = 'tv_show' AND item_id = old.id;
END;

-- playlists
CREATE TRIGGER trg_playlists_ai AFTER INSERT ON playlists BEGIN
    INSERT INTO search_index(rowid, kind, ref_id, title, subtitle)
    VALUES (new.id * 8 + 6, 'playlist', new.id, new.name, COALESCE(new.description, ''));
END;
CREATE TRIGGER trg_playlists_au AFTER UPDATE OF name, description ON playlists BEGIN
    DELETE FROM search_index WHERE rowid = old.id * 8 + 6;
    INSERT INTO search_index(rowid, kind, ref_id, title, subtitle)
    VALUES (new.id * 8 + 6, 'playlist', new.id, new.name, COALESCE(new.description, ''));
END;
CREATE TRIGGER trg_playlists_ad AFTER DELETE ON playlists BEGIN
    DELETE FROM search_index WHERE rowid = old.id * 8 + 6;
END;
