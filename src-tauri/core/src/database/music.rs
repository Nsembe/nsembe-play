//! Read queries for the music library.

use std::collections::HashMap;

use crate::database::{clamp_limit, fav_expr, placeholders, Db};
use crate::errors::{CoreError, Result};
use crate::models::{Album, AlbumDetail, Artist, ArtistDetail, Genre, Page, Track};

fn track_select() -> String {
    format!(
        "SELECT t.id, t.media_id, t.title, t.artist_id, ar.name AS artist_name, t.album_id, al.title AS album_title,
                t.genre, t.year, t.track_number, t.disc_number, m.duration, al.artwork_thumb AS artwork,
                m.added_at, {fav} AS is_favorite
         FROM tracks t
         JOIN media m ON m.id = t.media_id
         LEFT JOIN artists ar ON ar.id = t.artist_id
         LEFT JOIN albums al ON al.id = t.album_id
         WHERE m.missing = 0",
        fav = fav_expr("track", "t.id")
    )
}

fn album_select() -> String {
    format!(
        "SELECT a.id, a.title, a.artist_id, ar.name AS artist_name, a.year, a.genre,
                a.artwork_thumb AS artwork, a.artwork_path AS artwork_full,
                (SELECT COUNT(*) FROM tracks t JOIN media m ON m.id = t.media_id
                    WHERE t.album_id = a.id AND m.missing = 0) AS track_count,
                (SELECT COALESCE(SUM(m.duration), 0.0) FROM tracks t JOIN media m ON m.id = t.media_id
                    WHERE t.album_id = a.id AND m.missing = 0) AS duration,
                a.added_at, {fav} AS is_favorite
         FROM albums a
         JOIN artists ar ON ar.id = a.artist_id
         WHERE EXISTS (SELECT 1 FROM tracks t JOIN media m ON m.id = t.media_id
                       WHERE t.album_id = a.id AND m.missing = 0)",
        fav = fav_expr("album", "a.id")
    )
}

fn artist_select() -> String {
    format!(
        "SELECT ar.id, ar.name,
                COALESCE(ar.artwork_path,
                    (SELECT al.artwork_thumb FROM albums al WHERE al.artist_id = ar.id AND al.artwork_thumb IS NOT NULL
                        ORDER BY al.year DESC LIMIT 1),
                    (SELECT al.artwork_thumb FROM tracks t JOIN albums al ON al.id = t.album_id
                        WHERE t.artist_id = ar.id AND al.artwork_thumb IS NOT NULL LIMIT 1)) AS artwork,
                (SELECT COUNT(*) FROM albums al WHERE al.artist_id = ar.id) AS album_count,
                (SELECT COUNT(*) FROM tracks t JOIN media m ON m.id = t.media_id
                    WHERE t.artist_id = ar.id AND m.missing = 0) AS track_count,
                {fav} AS is_favorite
         FROM artists ar
         WHERE EXISTS (SELECT 1 FROM tracks t JOIN media m ON m.id = t.media_id
                       LEFT JOIN albums al ON al.id = t.album_id
                       WHERE (t.artist_id = ar.id OR al.artist_id = ar.id) AND m.missing = 0)",
        fav = fav_expr("artist", "ar.id")
    )
}

fn track_order(sort: &str) -> &'static str {
    match sort {
        "artist" => "ar.name COLLATE NOCASE, al.title COLLATE NOCASE, t.disc_number, t.track_number",
        "album" => "al.title COLLATE NOCASE, t.disc_number, t.track_number",
        "added" => "m.added_at DESC, t.id DESC",
        "duration" => "m.duration DESC",
        _ => "t.title COLLATE NOCASE, t.id",
    }
}

fn album_order(sort: &str) -> &'static str {
    match sort {
        "artist" => "ar.name COLLATE NOCASE, a.year, a.title COLLATE NOCASE",
        "added" => "a.added_at DESC, a.id DESC",
        "year" => "a.year DESC, a.title COLLATE NOCASE",
        _ => "a.title COLLATE NOCASE, a.id",
    }
}

async fn count_tracks(db: &Db, extra_where: &str, bind: Option<&str>) -> Result<i64> {
    let sql = format!(
        "SELECT COUNT(*) FROM tracks t JOIN media m ON m.id = t.media_id WHERE m.missing = 0 {extra_where}"
    );
    let mut q = sqlx::query_scalar::<_, i64>(&sql);
    if let Some(b) = bind {
        q = q.bind(b);
    }
    Ok(q.fetch_one(db.pool()).await?)
}

pub async fn list_tracks(db: &Db, sort: &str, offset: i64, limit: i64) -> Result<Page<Track>> {
    let limit = clamp_limit(limit);
    let sql = format!("{} ORDER BY {} LIMIT ? OFFSET ?", track_select(), track_order(sort));
    let items = sqlx::query_as::<_, Track>(&sql).bind(limit).bind(offset).fetch_all(db.pool()).await?;
    let total = count_tracks(db, "", None).await?;
    Ok(Page { items, total, offset, limit })
}

pub async fn genre_tracks(db: &Db, genre: &str, offset: i64, limit: i64) -> Result<Page<Track>> {
    let limit = clamp_limit(limit);
    let sql = format!(
        "{} AND t.genre = ? COLLATE NOCASE ORDER BY ar.name COLLATE NOCASE, al.title, t.disc_number, t.track_number LIMIT ? OFFSET ?",
        track_select()
    );
    let items = sqlx::query_as::<_, Track>(&sql).bind(genre).bind(limit).bind(offset).fetch_all(db.pool()).await?;
    let total = count_tracks(db, "AND t.genre = ? COLLATE NOCASE", Some(genre)).await?;
    Ok(Page { items, total, offset, limit })
}

pub async fn album_tracks(db: &Db, album_id: i64) -> Result<Vec<Track>> {
    let sql = format!(
        "{} AND t.album_id = ? ORDER BY COALESCE(t.disc_number, 1), COALESCE(t.track_number, 9999), t.title COLLATE NOCASE",
        track_select()
    );
    Ok(sqlx::query_as::<_, Track>(&sql).bind(album_id).fetch_all(db.pool()).await?)
}

pub async fn artist_tracks(db: &Db, artist_id: i64, limit: i64) -> Result<Vec<Track>> {
    // "Top" tracks: most played first, then album order.
    let sql = format!(
        "{} AND (t.artist_id = ? OR al.artist_id = ?)
         ORDER BY (SELECT COALESCE(p.play_count, 0) FROM playback_progress p WHERE p.media_id = t.media_id) DESC,
                  al.year DESC, t.disc_number, t.track_number
         LIMIT ?",
        track_select()
    );
    Ok(sqlx::query_as::<_, Track>(&sql)
        .bind(artist_id)
        .bind(artist_id)
        .bind(clamp_limit(limit))
        .fetch_all(db.pool())
        .await?)
}

/// Fetches tracks by id preserving the order of `ids`.
pub async fn tracks_by_ids(db: &Db, ids: &[i64]) -> Result<Vec<Track>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let sql = format!("{} AND t.id IN ({})", track_select(), placeholders(ids.len()));
    let mut q = sqlx::query_as::<_, Track>(&sql);
    for id in ids {
        q = q.bind(id);
    }
    let rows = q.fetch_all(db.pool()).await?;
    Ok(reorder(rows, ids, |t| t.id))
}

pub async fn list_albums(db: &Db, sort: &str, offset: i64, limit: i64) -> Result<Page<Album>> {
    let limit = clamp_limit(limit);
    let sql = format!("{} ORDER BY {} LIMIT ? OFFSET ?", album_select(), album_order(sort));
    let items = sqlx::query_as::<_, Album>(&sql).bind(limit).bind(offset).fetch_all(db.pool()).await?;
    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM albums a WHERE EXISTS (SELECT 1 FROM tracks t JOIN media m ON m.id = t.media_id
            WHERE t.album_id = a.id AND m.missing = 0)",
    )
    .fetch_one(db.pool())
    .await?;
    Ok(Page { items, total, offset, limit })
}

pub async fn albums_by_ids(db: &Db, ids: &[i64]) -> Result<Vec<Album>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let sql = format!("{} AND a.id IN ({})", album_select(), placeholders(ids.len()));
    let mut q = sqlx::query_as::<_, Album>(&sql);
    for id in ids {
        q = q.bind(id);
    }
    let rows = q.fetch_all(db.pool()).await?;
    Ok(reorder(rows, ids, |a| a.id))
}

pub async fn album(db: &Db, id: i64) -> Result<Album> {
    let sql = format!("{} AND a.id = ?", album_select());
    sqlx::query_as::<_, Album>(&sql)
        .bind(id)
        .fetch_optional(db.pool())
        .await?
        .ok_or_else(|| CoreError::not_found("Album not found"))
}

pub async fn album_detail(db: &Db, id: i64) -> Result<AlbumDetail> {
    let album = album(db, id).await?;
    let tracks = album_tracks(db, id).await?;
    let sql = format!("{} AND a.artist_id = ? AND a.id <> ? ORDER BY a.year DESC LIMIT 12", album_select());
    let more_by_artist = sqlx::query_as::<_, Album>(&sql)
        .bind(album.artist_id)
        .bind(id)
        .fetch_all(db.pool())
        .await?;
    Ok(AlbumDetail { album, tracks, more_by_artist })
}

pub async fn list_artists(db: &Db, offset: i64, limit: i64) -> Result<Page<Artist>> {
    let limit = clamp_limit(limit);
    let sql = format!("{} ORDER BY COALESCE(ar.sort_name, ar.name) COLLATE NOCASE LIMIT ? OFFSET ?", artist_select());
    let items = sqlx::query_as::<_, Artist>(&sql).bind(limit).bind(offset).fetch_all(db.pool()).await?;
    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM artists ar WHERE EXISTS (SELECT 1 FROM tracks t JOIN media m ON m.id = t.media_id
            LEFT JOIN albums al ON al.id = t.album_id
            WHERE (t.artist_id = ar.id OR al.artist_id = ar.id) AND m.missing = 0)",
    )
    .fetch_one(db.pool())
    .await?;
    Ok(Page { items, total, offset, limit })
}

pub async fn artists_by_ids(db: &Db, ids: &[i64]) -> Result<Vec<Artist>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let sql = format!("{} AND ar.id IN ({})", artist_select(), placeholders(ids.len()));
    let mut q = sqlx::query_as::<_, Artist>(&sql);
    for id in ids {
        q = q.bind(id);
    }
    let rows = q.fetch_all(db.pool()).await?;
    Ok(reorder(rows, ids, |a| a.id))
}

pub async fn artist_detail(db: &Db, id: i64) -> Result<ArtistDetail> {
    let artist = artists_by_ids(db, &[id])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| CoreError::not_found("Artist not found"))?;
    let sql = format!(
        "{} AND (a.artist_id = ? OR EXISTS (SELECT 1 FROM tracks t WHERE t.album_id = a.id AND t.artist_id = ?))
         ORDER BY a.year DESC, a.title COLLATE NOCASE",
        album_select()
    );
    let albums = sqlx::query_as::<_, Album>(&sql).bind(id).bind(id).fetch_all(db.pool()).await?;
    let top_tracks = artist_tracks(db, id, 50).await?;
    Ok(ArtistDetail { artist, albums, top_tracks })
}

pub async fn genres(db: &Db) -> Result<Vec<Genre>> {
    Ok(sqlx::query_as::<_, Genre>(
        "SELECT t.genre AS name, COUNT(*) AS track_count,
                (SELECT al.artwork_thumb FROM tracks t2 JOIN albums al ON al.id = t2.album_id
                 WHERE t2.genre = t.genre AND al.artwork_thumb IS NOT NULL LIMIT 1) AS artwork
         FROM tracks t JOIN media m ON m.id = t.media_id
         WHERE m.missing = 0 AND t.genre IS NOT NULL AND t.genre <> ''
         GROUP BY t.genre COLLATE NOCASE
         ORDER BY track_count DESC",
    )
    .fetch_all(db.pool())
    .await?)
}

/// Random selection of songs for "shuffle all".
pub async fn random_track_media_ids(db: &Db, limit: i64) -> Result<Vec<i64>> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT t.media_id FROM tracks t JOIN media m ON m.id = t.media_id WHERE m.missing = 0 ORDER BY RANDOM() LIMIT ?",
    )
    .bind(clamp_limit(limit))
    .fetch_all(db.pool())
    .await?)
}

/// Media ids of an album in play order.
pub async fn album_media_ids(db: &Db, album_id: i64) -> Result<Vec<i64>> {
    Ok(album_tracks(db, album_id).await?.into_iter().map(|t| t.media_id).collect())
}

pub(crate) fn reorder<T>(rows: Vec<T>, ids: &[i64], key: impl Fn(&T) -> i64) -> Vec<T> {
    let pos: HashMap<i64, usize> = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let mut rows = rows;
    rows.sort_by_key(|r| pos.get(&key(r)).copied().unwrap_or(usize::MAX));
    rows
}
