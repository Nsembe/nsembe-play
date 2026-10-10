//! Write-side operations used by the scanner and filesystem watcher.

use std::collections::HashMap;

use sqlx::SqliteConnection;

use crate::database::Db;
use crate::errors::Result;
use crate::models::{MediaKind, StreamInfo};
use crate::util::{now, separator};

/// Technical properties of a media file.
#[derive(Debug, Clone, Default)]
pub struct TechInfo {
    pub duration: Option<f64>,
    pub container: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub bitrate: Option<i64>,
    pub audio_streams: Vec<StreamInfo>,
    pub subtitle_streams: Vec<StreamInfo>,
}

#[derive(Debug, Clone)]
pub struct FileRecord {
    pub folder_id: i64,
    pub path: String,
    pub kind: MediaKind,
    pub size: i64,
    pub mtime: i64,
    pub fingerprint: Option<String>,
    pub tech: TechInfo,
    pub scan_error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TrackRecord {
    pub title: String,
    pub artist: String,
    pub album_artist: Option<String>,
    pub album: String,
    pub genre: Option<String>,
    pub year: Option<i64>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct MovieRecord {
    pub title: String,
    pub year: Option<i64>,
    pub genres: Option<String>,
    pub overview: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EpisodeRecord {
    pub show_title: String,
    pub show_year: Option<i64>,
    pub show_folder: Option<String>,
    pub season_number: i64,
    pub season_folder: Option<String>,
    pub episode_number: i64,
    pub title: String,
    pub overview: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExistingMedia {
    pub id: i64,
    pub size: i64,
    pub mtime: i64,
    pub missing: bool,
    pub has_error: bool,
}

fn json_or_null(streams: &[StreamInfo]) -> Option<String> {
    if streams.is_empty() {
        None
    } else {
        serde_json::to_string(streams).ok()
    }
}

/// Inserts or updates the `media` row for a file. Returns `(media_id, inserted)`.
pub async fn upsert_media(conn: &mut SqliteConnection, f: &FileRecord) -> Result<(i64, bool)> {
    let ts = now();
    let existing = sqlx::query_scalar::<_, i64>("SELECT id FROM media WHERE path = ?")
        .bind(&f.path)
        .fetch_optional(&mut *conn)
        .await?;
    let audio = json_or_null(&f.tech.audio_streams);
    let subs = json_or_null(&f.tech.subtitle_streams);
    match existing {
        Some(id) => {
            sqlx::query(
                "UPDATE media SET folder_id = ?, kind = ?, file_size = ?, file_mtime = ?, fingerprint = ?,
                    duration = COALESCE(?, duration), container = ?, video_codec = ?, audio_codec = ?,
                    width = ?, height = ?, bitrate = ?, audio_streams = ?, subtitle_streams = ?,
                    artwork_checked = 0, missing = 0, missing_since = NULL, scan_error = ?, updated_at = ?
                 WHERE id = ?",
            )
            .bind(f.folder_id)
            .bind(f.kind.as_str())
            .bind(f.size)
            .bind(f.mtime)
            .bind(&f.fingerprint)
            .bind(f.tech.duration)
            .bind(&f.tech.container)
            .bind(&f.tech.video_codec)
            .bind(&f.tech.audio_codec)
            .bind(f.tech.width)
            .bind(f.tech.height)
            .bind(f.tech.bitrate)
            .bind(audio)
            .bind(subs)
            .bind(&f.scan_error)
            .bind(ts)
            .bind(id)
            .execute(&mut *conn)
            .await?;
            Ok((id, false))
        }
        None => {
            let id = sqlx::query_scalar::<_, i64>(
                "INSERT INTO media(folder_id, path, kind, file_size, file_mtime, fingerprint, duration, container,
                    video_codec, audio_codec, width, height, bitrate, audio_streams, subtitle_streams, scan_error,
                    added_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
            )
            .bind(f.folder_id)
            .bind(&f.path)
            .bind(f.kind.as_str())
            .bind(f.size)
            .bind(f.mtime)
            .bind(&f.fingerprint)
            .bind(f.tech.duration)
            .bind(&f.tech.container)
            .bind(&f.tech.video_codec)
            .bind(&f.tech.audio_codec)
            .bind(f.tech.width)
            .bind(f.tech.height)
            .bind(f.tech.bitrate)
            .bind(audio)
            .bind(subs)
            .bind(&f.scan_error)
            .bind(ts)
            .bind(ts)
            .fetch_one(&mut *conn)
            .await?;
            Ok((id, true))
        }
    }
}

pub async fn get_or_create_artist(conn: &mut SqliteConnection, name: &str) -> Result<i64> {
    Ok(sqlx::query_scalar::<_, i64>(
        "INSERT INTO artists(name, sort_name, added_at) VALUES (?, ?, ?)
         ON CONFLICT(name) DO UPDATE SET name = artists.name RETURNING id",
    )
    .bind(name)
    .bind(sort_key(name))
    .bind(now())
    .fetch_one(&mut *conn)
    .await?)
}

pub async fn get_or_create_album(
    conn: &mut SqliteConnection,
    title: &str,
    artist_id: i64,
    year: Option<i64>,
    genre: Option<&str>,
) -> Result<i64> {
    Ok(sqlx::query_scalar::<_, i64>(
        "INSERT INTO albums(title, artist_id, year, genre, added_at) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(title, artist_id) DO UPDATE SET
            year = COALESCE(albums.year, excluded.year),
            genre = COALESCE(albums.genre, excluded.genre)
         RETURNING id",
    )
    .bind(title)
    .bind(artist_id)
    .bind(year)
    .bind(genre)
    .bind(now())
    .fetch_one(&mut *conn)
    .await?)
}

pub async fn save_track(conn: &mut SqliteConnection, media_id: i64, t: &TrackRecord) -> Result<i64> {
    let artist_id = get_or_create_artist(conn, &t.artist).await?;
    let album_artist_id = match t.album_artist.as_deref() {
        Some(name) if !name.eq_ignore_ascii_case(&t.artist) => get_or_create_artist(conn, name).await?,
        _ => artist_id,
    };
    let album_id = get_or_create_album(conn, &t.album, album_artist_id, t.year, t.genre.as_deref()).await?;
    // A changed track may bring new artwork (e.g. cover.jpg added later).
    sqlx::query("UPDATE albums SET artwork_checked = 0 WHERE id = ? AND artwork_path IS NULL")
        .bind(album_id)
        .execute(&mut *conn)
        .await?;
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO tracks(media_id, title, artist_id, album_id, genre, year, track_number, disc_number)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(media_id) DO UPDATE SET title = excluded.title, artist_id = excluded.artist_id,
            album_id = excluded.album_id, genre = excluded.genre, year = excluded.year,
            track_number = excluded.track_number, disc_number = excluded.disc_number
         RETURNING id",
    )
    .bind(media_id)
    .bind(&t.title)
    .bind(artist_id)
    .bind(album_id)
    .bind(&t.genre)
    .bind(t.year)
    .bind(t.track_number)
    .bind(t.disc_number)
    .fetch_one(&mut *conn)
    .await?;
    Ok(id)
}

pub async fn save_movie(conn: &mut SqliteConnection, media_id: i64, m: &MovieRecord) -> Result<i64> {
    Ok(sqlx::query_scalar::<_, i64>(
        "INSERT INTO movies(media_id, title, sort_title, year, genres, overview, added_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(media_id) DO UPDATE SET title = excluded.title, sort_title = excluded.sort_title,
            year = excluded.year, genres = COALESCE(excluded.genres, movies.genres),
            overview = COALESCE(excluded.overview, movies.overview)
         RETURNING id",
    )
    .bind(media_id)
    .bind(&m.title)
    .bind(sort_key(&m.title))
    .bind(m.year)
    .bind(&m.genres)
    .bind(&m.overview)
    .bind(now())
    .fetch_one(&mut *conn)
    .await?)
}

pub async fn save_episode(conn: &mut SqliteConnection, media_id: i64, e: &EpisodeRecord) -> Result<i64> {
    let show_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO tv_shows(title, folder_path, year, added_at) VALUES (?, ?, ?, ?)
         ON CONFLICT(title) DO UPDATE SET folder_path = COALESCE(tv_shows.folder_path, excluded.folder_path),
            year = COALESCE(tv_shows.year, excluded.year)
         RETURNING id",
    )
    .bind(&e.show_title)
    .bind(&e.show_folder)
    .bind(e.show_year)
    .bind(now())
    .fetch_one(&mut *conn)
    .await?;
    let season_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO tv_seasons(show_id, season_number, folder_path) VALUES (?, ?, ?)
         ON CONFLICT(show_id, season_number) DO UPDATE SET folder_path = COALESCE(tv_seasons.folder_path, excluded.folder_path)
         RETURNING id",
    )
    .bind(show_id)
    .bind(e.season_number)
    .bind(&e.season_folder)
    .fetch_one(&mut *conn)
    .await?;
    sqlx::query("UPDATE tv_shows SET artwork_checked = 0 WHERE id = ? AND poster_path IS NULL")
        .bind(show_id)
        .execute(&mut *conn)
        .await?;
    Ok(sqlx::query_scalar::<_, i64>(
        "INSERT INTO tv_episodes(media_id, show_id, season_id, season_number, episode_number, title, overview)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(media_id) DO UPDATE SET show_id = excluded.show_id, season_id = excluded.season_id,
            season_number = excluded.season_number, episode_number = excluded.episode_number,
            title = excluded.title, overview = COALESCE(excluded.overview, tv_episodes.overview)
         RETURNING id",
    )
    .bind(media_id)
    .bind(show_id)
    .bind(season_id)
    .bind(e.season_number)
    .bind(e.episode_number)
    .bind(&e.title)
    .bind(&e.overview)
    .fetch_one(&mut *conn)
    .await?)
}

/// Removes kind-specific rows that no longer match the media kind (e.g. a folder's
/// type was changed). Keeps the database consistent when re-indexing.
pub async fn clear_other_kinds(conn: &mut SqliteConnection, media_id: i64, kind: MediaKind) -> Result<()> {
    if kind != MediaKind::Track {
        sqlx::query("DELETE FROM tracks WHERE media_id = ?").bind(media_id).execute(&mut *conn).await?;
    }
    if kind != MediaKind::Movie {
        sqlx::query("DELETE FROM movies WHERE media_id = ?").bind(media_id).execute(&mut *conn).await?;
    }
    if kind != MediaKind::Episode {
        sqlx::query("DELETE FROM tv_episodes WHERE media_id = ?").bind(media_id).execute(&mut *conn).await?;
    }
    Ok(())
}

/// Known files of a folder keyed by path, used for incremental scans.
pub async fn existing_for_folder(db: &Db, folder_id: i64) -> Result<HashMap<String, ExistingMedia>> {
    let rows = sqlx::query_as::<_, (i64, String, i64, i64, bool, bool)>(
        "SELECT id, path, file_size, file_mtime, missing, scan_error IS NOT NULL FROM media WHERE folder_id = ?",
    )
    .bind(folder_id)
    .fetch_all(db.pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, path, size, mtime, missing, has_error)| (path, ExistingMedia { id, size, mtime, missing, has_error }))
        .collect())
}

pub async fn find_by_path(db: &Db, path: &str) -> Result<Option<ExistingMedia>> {
    let row = sqlx::query_as::<_, (i64, i64, i64, bool, bool)>(
        "SELECT id, file_size, file_mtime, missing, scan_error IS NOT NULL FROM media WHERE path = ?",
    )
    .bind(path)
    .fetch_optional(db.pool())
    .await?;
    Ok(row.map(|(id, size, mtime, missing, has_error)| ExistingMedia { id, size, mtime, missing, has_error }))
}

/// Media rows with the same content fingerprint (move/rename detection).
pub async fn find_by_fingerprint(db: &Db, fingerprint: &str, size: i64) -> Result<Vec<(i64, String)>> {
    Ok(sqlx::query_as::<_, (i64, String)>(
        "SELECT id, path FROM media WHERE fingerprint = ? AND file_size = ? ORDER BY missing DESC",
    )
    .bind(fingerprint)
    .bind(size)
    .fetch_all(db.pool())
    .await?)
}

/// Points an existing media row at a new location, preserving history, playlists and favorites.
pub async fn relink(db: &Db, media_id: i64, new_path: &str, folder_id: i64) -> Result<()> {
    let mut tx = db.pool().begin().await?;
    sqlx::query("DELETE FROM media WHERE path = ? AND id <> ?")
        .bind(new_path)
        .bind(media_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE media SET path = ?, folder_id = ?, missing = 0, missing_since = NULL, updated_at = ? WHERE id = ?")
        .bind(new_path)
        .bind(folder_id)
        .bind(now())
        .bind(media_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Marks media at `path` - or anywhere below it when it was a directory - as missing.
pub async fn mark_missing_under(db: &Db, path: &str) -> Result<u64> {
    let prefix = format!("{path}{}", separator());
    let res = sqlx::query(
        "UPDATE media SET missing = 1, missing_since = COALESCE(missing_since, ?)
         WHERE missing = 0 AND (path = ? OR substr(path, 1, length(?)) = ?)",
    )
    .bind(now())
    .bind(path)
    .bind(&prefix)
    .bind(&prefix)
    .execute(db.pool())
    .await?;
    Ok(res.rows_affected())
}

pub async fn mark_missing_ids(db: &Db, ids: &[i64]) -> Result<u64> {
    let mut total = 0;
    for chunk in ids.chunks(400) {
        let sql = format!(
            "UPDATE media SET missing = 1, missing_since = COALESCE(missing_since, ?) WHERE id IN ({})",
            super::placeholders(chunk.len())
        );
        let mut q = sqlx::query(&sql).bind(now());
        for id in chunk {
            q = q.bind(id);
        }
        total += q.execute(db.pool()).await?.rows_affected();
    }
    Ok(total)
}

/// Applies a rename of a file or directory. Returns the media rows that moved
/// together with their new paths so they can be re-indexed.
pub async fn rename_under(db: &Db, from: &str, to: &str, folder_id: i64) -> Result<Vec<(i64, String)>> {
    let from_prefix = format!("{from}{}", separator());
    let to_prefix = format!("{to}{}", separator());
    let mut tx = db.pool().begin().await?;
    // Stale rows already registered at the destination would violate UNIQUE(path).
    sqlx::query("DELETE FROM media WHERE path = ? OR substr(path, 1, length(?)) = ?")
        .bind(to)
        .bind(&to_prefix)
        .bind(&to_prefix)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE media SET path = ? || substr(path, length(?) + 1), folder_id = ?, missing = 0,
            missing_since = NULL, updated_at = ?
         WHERE path = ? OR substr(path, 1, length(?)) = ?",
    )
    .bind(to)
    .bind(from)
    .bind(folder_id)
    .bind(now())
    .bind(from)
    .bind(&from_prefix)
    .bind(&from_prefix)
    .execute(&mut *tx)
    .await?;
    let moved = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, path FROM media WHERE path = ? OR substr(path, 1, length(?)) = ?",
    )
    .bind(to)
    .bind(&to_prefix)
    .bind(&to_prefix)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(moved)
}

pub async fn set_missing(db: &Db, media_id: i64) -> Result<()> {
    sqlx::query("UPDATE media SET missing = 1, missing_since = COALESCE(missing_since, ?) WHERE id = ?")
        .bind(now())
        .bind(media_id)
        .execute(db.pool())
        .await?;
    Ok(())
}

/// Fills in a duration discovered at playback time (e.g. when ffprobe is unavailable).
pub async fn update_duration_if_unknown(db: &Db, media_id: i64, duration: f64) -> Result<()> {
    sqlx::query("UPDATE media SET duration = ? WHERE id = ? AND (duration IS NULL OR duration <= 0)")
        .bind(duration)
        .bind(media_id)
        .execute(db.pool())
        .await?;
    Ok(())
}

/// Deletes albums, artists, seasons and shows that no longer have any media.
pub async fn prune_orphans(db: &Db) -> Result<()> {
    let mut tx = db.pool().begin().await?;
    for sql in [
        "DELETE FROM albums WHERE NOT EXISTS (SELECT 1 FROM tracks t WHERE t.album_id = albums.id)",
        "DELETE FROM artists WHERE NOT EXISTS (SELECT 1 FROM tracks t WHERE t.artist_id = artists.id)
            AND NOT EXISTS (SELECT 1 FROM albums a WHERE a.artist_id = artists.id)",
        "DELETE FROM tv_seasons WHERE NOT EXISTS (SELECT 1 FROM tv_episodes e WHERE e.season_id = tv_seasons.id)",
        "DELETE FROM tv_shows WHERE NOT EXISTS (SELECT 1 FROM tv_episodes e WHERE e.show_id = tv_shows.id)",
    ] {
        sqlx::query(sql).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Permanently forgets missing media. `older_than` limits it to items missing since before that time.
pub async fn purge_missing(db: &Db, older_than: Option<i64>) -> Result<u64> {
    let res = match older_than {
        Some(ts) => sqlx::query("DELETE FROM media WHERE missing = 1 AND missing_since < ?").bind(ts).execute(db.pool()).await?,
        None => sqlx::query("DELETE FROM media WHERE missing = 1").execute(db.pool()).await?,
    };
    if res.rows_affected() > 0 {
        prune_orphans(db).await?;
    }
    Ok(res.rows_affected())
}

/// Sort key ignoring leading articles: "The Matrix" -> "matrix".
pub fn sort_key(title: &str) -> String {
    let lower = title.trim().to_lowercase();
    for article in ["the ", "a ", "an "] {
        if let Some(rest) = lower.strip_prefix(article) {
            if !rest.is_empty() {
                return rest.to_string();
            }
        }
    }
    lower
}
