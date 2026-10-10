//! User playlists: ordered lists of media items (songs, movies or episodes).

use std::collections::HashMap;

use sqlx::FromRow;

use crate::database::playback::playables;
use crate::database::Db;
use crate::errors::{CoreError, Result};
use crate::models::{Playlist, PlaylistDetail, PlaylistEntry};
use crate::util::now;

#[derive(FromRow)]
struct PlaylistRow {
    id: i64,
    name: String,
    description: Option<String>,
    artwork_path: Option<String>,
    created_at: i64,
    updated_at: i64,
    item_count: i64,
    duration: f64,
}

const PLAYLIST_SELECT: &str = "
    SELECT p.id, p.name, p.description, p.artwork_path, p.created_at, p.updated_at,
           (SELECT COUNT(*) FROM playlist_items pi JOIN media m ON m.id = pi.media_id
               WHERE pi.playlist_id = p.id AND m.missing = 0) AS item_count,
           (SELECT COALESCE(SUM(m.duration), 0.0) FROM playlist_items pi JOIN media m ON m.id = pi.media_id
               WHERE pi.playlist_id = p.id AND m.missing = 0) AS duration
    FROM playlists p";

async fn hydrate(db: &Db, row: PlaylistRow) -> Result<Playlist> {
    let cover_artworks = sqlx::query_scalar::<_, String>(
        "SELECT art FROM (
            SELECT COALESCE(al.artwork_thumb, mv.poster_thumb, mv.backdrop_path, m.thumb_path, s.poster_thumb) AS art,
                   MIN(pi.position) AS pos
            FROM playlist_items pi
            JOIN media m ON m.id = pi.media_id
            LEFT JOIN tracks t ON t.media_id = m.id
            LEFT JOIN albums al ON al.id = t.album_id
            LEFT JOIN movies mv ON mv.media_id = m.id
            LEFT JOIN tv_episodes e ON e.media_id = m.id
            LEFT JOIN tv_shows s ON s.id = e.show_id
            WHERE pi.playlist_id = ? AND m.missing = 0
            GROUP BY art
         ) WHERE art IS NOT NULL ORDER BY pos LIMIT 4",
    )
    .bind(row.id)
    .fetch_all(db.pool())
    .await?;
    Ok(Playlist {
        id: row.id,
        name: row.name,
        description: row.description,
        artwork: row.artwork_path,
        created_at: row.created_at,
        updated_at: row.updated_at,
        item_count: row.item_count,
        duration: row.duration,
        cover_artworks,
    })
}

pub async fn list(db: &Db) -> Result<Vec<Playlist>> {
    let rows = sqlx::query_as::<_, PlaylistRow>(&format!("{PLAYLIST_SELECT} ORDER BY p.updated_at DESC, p.id DESC"))
        .fetch_all(db.pool())
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(hydrate(db, row).await?);
    }
    Ok(out)
}

pub async fn by_ids(db: &Db, ids: &[i64]) -> Result<Vec<Playlist>> {
    let mut out = Vec::new();
    for id in ids {
        if let Ok(p) = summary(db, *id).await {
            out.push(p);
        }
    }
    Ok(out)
}

pub async fn summary(db: &Db, id: i64) -> Result<Playlist> {
    let row = sqlx::query_as::<_, PlaylistRow>(&format!("{PLAYLIST_SELECT} WHERE p.id = ?"))
        .bind(id)
        .fetch_optional(db.pool())
        .await?
        .ok_or_else(|| CoreError::not_found("Playlist not found"))?;
    hydrate(db, row).await
}

pub async fn detail(db: &Db, id: i64) -> Result<PlaylistDetail> {
    let playlist = summary(db, id).await?;
    let rows = sqlx::query_as::<_, (i64, i64, i64, i64)>(
        "SELECT id, media_id, position, added_at FROM playlist_items WHERE playlist_id = ? ORDER BY position, id",
    )
    .bind(id)
    .fetch_all(db.pool())
    .await?;
    let media_ids: Vec<i64> = rows.iter().map(|r| r.1).collect();
    let media: HashMap<i64, _> = playables(db, &media_ids).await?.into_iter().map(|p| (p.media_id, p)).collect();
    let entries = rows
        .into_iter()
        .filter_map(|(item_id, media_id, position, added_at)| {
            media.get(&media_id).cloned().map(|media| PlaylistEntry { item_id, position, added_at, media })
        })
        .collect();
    Ok(PlaylistDetail { playlist, entries })
}

fn validate_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::invalid("Playlist name cannot be empty"));
    }
    if name.chars().count() > 120 {
        return Err(CoreError::invalid("Playlist name is too long"));
    }
    Ok(name.to_string())
}

fn clean_description(description: Option<&str>) -> Option<String> {
    description.map(str::trim).filter(|d| !d.is_empty()).map(str::to_string)
}

pub async fn create(db: &Db, name: &str, description: Option<&str>) -> Result<Playlist> {
    let name = validate_name(name)?;
    let ts = now();
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO playlists(name, description, created_at, updated_at) VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(name)
    .bind(clean_description(description))
    .bind(ts)
    .bind(ts)
    .fetch_one(db.pool())
    .await?;
    summary(db, id).await
}

pub async fn update(db: &Db, id: i64, name: &str, description: Option<&str>) -> Result<Playlist> {
    let name = validate_name(name)?;
    let res = sqlx::query("UPDATE playlists SET name = ?, description = ?, updated_at = ? WHERE id = ?")
        .bind(name)
        .bind(clean_description(description))
        .bind(now())
        .bind(id)
        .execute(db.pool())
        .await?;
    if res.rows_affected() == 0 {
        return Err(CoreError::not_found("Playlist not found"));
    }
    summary(db, id).await
}

pub async fn set_artwork(db: &Db, id: i64, artwork: Option<&str>) -> Result<()> {
    sqlx::query("UPDATE playlists SET artwork_path = ?, updated_at = ? WHERE id = ?")
        .bind(artwork)
        .bind(now())
        .bind(id)
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn delete(db: &Db, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM playlists WHERE id = ?").bind(id).execute(db.pool()).await?;
    Ok(())
}

/// Appends items to the end of a playlist, skipping ones already present.
/// Returns the number of items added.
pub async fn add_items(db: &Db, id: i64, media_ids: &[i64]) -> Result<u64> {
    let mut tx = db.pool().begin().await?;
    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM playlists WHERE id = ?")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    if exists == 0 {
        return Err(CoreError::not_found("Playlist not found"));
    }
    let mut next = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_items WHERE playlist_id = ?",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    let ts = now();
    let mut added = 0;
    for media_id in media_ids {
        let present = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM playlist_items WHERE playlist_id = ? AND media_id = ?",
        )
        .bind(id)
        .bind(media_id)
        .fetch_one(&mut *tx)
        .await?;
        if present > 0 {
            continue;
        }
        sqlx::query("INSERT INTO playlist_items(playlist_id, media_id, position, added_at) VALUES (?, ?, ?, ?)")
            .bind(id)
            .bind(media_id)
            .bind(next)
            .bind(ts)
            .execute(&mut *tx)
            .await?;
        next += 1;
        added += 1;
    }
    sqlx::query("UPDATE playlists SET updated_at = ? WHERE id = ?").bind(ts).bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(added)
}

pub async fn remove_item(db: &Db, item_id: i64) -> Result<()> {
    let mut tx = db.pool().begin().await?;
    let playlist_id = sqlx::query_scalar::<_, i64>("DELETE FROM playlist_items WHERE id = ? RETURNING playlist_id")
        .bind(item_id)
        .fetch_optional(&mut *tx)
        .await?;
    if let Some(pid) = playlist_id {
        sqlx::query("UPDATE playlists SET updated_at = ? WHERE id = ?")
            .bind(now())
            .bind(pid)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Rewrites item positions to match `item_ids`. Items not listed keep their
/// relative order after the listed ones.
pub async fn reorder(db: &Db, playlist_id: i64, item_ids: &[i64]) -> Result<()> {
    let mut tx = db.pool().begin().await?;
    let current = sqlx::query_scalar::<_, i64>("SELECT id FROM playlist_items WHERE playlist_id = ? ORDER BY position, id")
        .bind(playlist_id)
        .fetch_all(&mut *tx)
        .await?;
    let mut order: Vec<i64> = item_ids.iter().copied().filter(|id| current.contains(id)).collect();
    order.extend(current.iter().copied().filter(|id| !item_ids.contains(id)));
    for (pos, item_id) in order.iter().enumerate() {
        sqlx::query("UPDATE playlist_items SET position = ? WHERE id = ? AND playlist_id = ?")
            .bind(pos as i64)
            .bind(item_id)
            .bind(playlist_id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE playlists SET updated_at = ? WHERE id = ?")
        .bind(now())
        .bind(playlist_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn media_ids(db: &Db, playlist_id: i64) -> Result<Vec<i64>> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT pi.media_id FROM playlist_items pi JOIN media m ON m.id = pi.media_id
         WHERE pi.playlist_id = ? AND m.missing = 0 ORDER BY pi.position, pi.id",
    )
    .bind(playlist_id)
    .fetch_all(db.pool())
    .await?)
}
