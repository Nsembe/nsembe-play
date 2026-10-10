//! Playback history, resume positions and "continue watching".

use std::collections::HashSet;

use crate::database::music::reorder;
use crate::database::{clamp_limit, placeholders, video, Db};
use crate::errors::{CoreError, Result};
use crate::models::{ContinueItem, Page, PlayableItem};
use crate::util::now;

/// Fraction of a video after which it counts as watched.
pub const COMPLETION_THRESHOLD: f64 = 0.90;
/// Items watched for less than this many seconds are not offered for resuming.
pub const MIN_RESUME_SECONDS: f64 = 30.0;

pub(crate) const PLAYABLE_SELECT: &str = "
    SELECT m.id AS media_id, m.kind, m.path, m.duration, m.missing,
           COALESCE(t.title, mv.title, e.title, m.path) AS title,
           CASE m.kind
               WHEN 'track' THEN COALESCE(ar.name, 'Unknown Artist')
               WHEN 'movie' THEN CAST(mv.year AS TEXT)
               ELSE s.title || ' · S' || printf('%02d', e.season_number) || 'E' || printf('%02d', e.episode_number)
           END AS subtitle,
           CASE m.kind
               WHEN 'track' THEN al.artwork_thumb
               WHEN 'movie' THEN COALESCE(mv.poster_thumb, mv.poster_path, mv.backdrop_path, m.thumb_path)
               ELSE COALESCE(m.thumb_path, s.poster_thumb, s.backdrop_path)
           END AS artwork,
           CASE m.kind
               WHEN 'track' THEN al.artwork_path
               WHEN 'movie' THEN COALESCE(mv.backdrop_path, m.thumb_path)
               ELSE COALESCE(s.backdrop_path, m.thumb_path)
           END AS backdrop,
           t.id AS track_id, t.album_id, al.title AS album_title, t.artist_id,
           mv.id AS movie_id, e.id AS episode_id, e.show_id,
           p.position, p.completed, p.last_played_at,
           CASE
               WHEN t.id IS NOT NULL THEN EXISTS(SELECT 1 FROM favorites f WHERE f.item_type = 'track' AND f.item_id = t.id)
               WHEN mv.id IS NOT NULL THEN EXISTS(SELECT 1 FROM favorites f WHERE f.item_type = 'movie' AND f.item_id = mv.id)
               ELSE 0
           END AS is_favorite
    FROM media m
    LEFT JOIN tracks t ON t.media_id = m.id
    LEFT JOIN artists ar ON ar.id = t.artist_id
    LEFT JOIN albums al ON al.id = t.album_id
    LEFT JOIN movies mv ON mv.media_id = m.id
    LEFT JOIN tv_episodes e ON e.media_id = m.id
    LEFT JOIN tv_shows s ON s.id = e.show_id
    LEFT JOIN playback_progress p ON p.media_id = m.id";

pub async fn playable(db: &Db, media_id: i64) -> Result<PlayableItem> {
    sqlx::query_as::<_, PlayableItem>(&format!("{PLAYABLE_SELECT} WHERE m.id = ?"))
        .bind(media_id)
        .fetch_optional(db.pool())
        .await?
        .ok_or_else(|| CoreError::not_found("Media item not found"))
}

/// Fetches playable items preserving the order of `ids` (missing files excluded).
pub async fn playables(db: &Db, ids: &[i64]) -> Result<Vec<PlayableItem>> {
    let mut out = Vec::with_capacity(ids.len());
    for chunk in ids.chunks(400) {
        let sql = format!("{PLAYABLE_SELECT} WHERE m.missing = 0 AND m.id IN ({})", placeholders(chunk.len()));
        let mut q = sqlx::query_as::<_, PlayableItem>(&sql);
        for id in chunk {
            q = q.bind(id);
        }
        out.extend(q.fetch_all(db.pool()).await?);
    }
    Ok(reorder(out, ids, |p| p.media_id))
}

/// Records the start of a play session (history entry + play count).
pub async fn record_play_start(db: &Db, media_id: i64, duration: Option<f64>) -> Result<()> {
    let ts = now();
    let mut tx = db.pool().begin().await?;
    sqlx::query("INSERT INTO play_history(media_id, played_at, position, duration) VALUES (?, ?, 0, ?)")
        .bind(media_id)
        .bind(ts)
        .bind(duration)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO playback_progress(media_id, position, duration, completed, play_count, last_played_at)
         VALUES (?, 0, ?, 0, 1, ?)
         ON CONFLICT(media_id) DO UPDATE SET play_count = play_count + 1, last_played_at = excluded.last_played_at,
            duration = COALESCE(excluded.duration, playback_progress.duration)",
    )
    .bind(media_id)
    .bind(duration)
    .bind(ts)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Whether a position counts as having finished the item.
pub fn is_completed(position: f64, duration: Option<f64>, ended: bool) -> bool {
    if ended {
        return true;
    }
    match duration {
        Some(d) if d > 0.0 => position / d >= COMPLETION_THRESHOLD,
        _ => false,
    }
}

/// Persists the current position of an item. `ended` marks natural end of playback.
pub async fn save_progress(db: &Db, media_id: i64, position: f64, duration: Option<f64>, ended: bool) -> Result<bool> {
    let duration = duration.filter(|d| d.is_finite() && *d > 0.0);
    let position = if position.is_finite() { position.max(0.0) } else { 0.0 };
    let completed = is_completed(position, duration, ended);
    let ts = now();
    sqlx::query(
        "INSERT INTO playback_progress(media_id, position, duration, completed, play_count, last_played_at)
         VALUES (?, ?, ?, ?, 1, ?)
         ON CONFLICT(media_id) DO UPDATE SET position = excluded.position,
            duration = COALESCE(excluded.duration, playback_progress.duration),
            completed = excluded.completed, last_played_at = excluded.last_played_at",
    )
    .bind(media_id)
    .bind(position)
    .bind(duration)
    .bind(completed)
    .bind(ts)
    .execute(db.pool())
    .await?;
    // Keep the most recent history entry in sync with how far the session got.
    sqlx::query(
        "UPDATE play_history SET position = ?, duration = COALESCE(?, duration)
         WHERE id = (SELECT id FROM play_history WHERE media_id = ? ORDER BY played_at DESC, id DESC LIMIT 1)",
    )
    .bind(position)
    .bind(duration)
    .bind(media_id)
    .execute(db.pool())
    .await?;
    if let Some(d) = duration {
        crate::database::media::update_duration_if_unknown(db, media_id, d).await?;
    }
    Ok(completed)
}

/// Marks an item watched/unwatched explicitly.
pub async fn set_watched(db: &Db, media_id: i64, watched: bool) -> Result<()> {
    if watched {
        sqlx::query(
            "INSERT INTO playback_progress(media_id, position, completed, play_count, last_played_at)
             VALUES (?, 0, 1, 0, ?)
             ON CONFLICT(media_id) DO UPDATE SET completed = 1, position = 0",
        )
        .bind(media_id)
        .bind(now())
        .execute(db.pool())
        .await?;
    } else {
        sqlx::query("UPDATE playback_progress SET completed = 0, position = 0 WHERE media_id = ?")
            .bind(media_id)
            .execute(db.pool())
            .await?;
    }
    Ok(())
}

/// Position to resume from, if the item was left part-way through.
pub async fn resume_position(db: &Db, media_id: i64) -> Result<Option<f64>> {
    let row = sqlx::query_as::<_, (f64, bool)>("SELECT position, completed FROM playback_progress WHERE media_id = ?")
        .bind(media_id)
        .fetch_optional(db.pool())
        .await?;
    Ok(match row {
        Some((pos, false)) if pos >= 5.0 => Some(pos),
        _ => None,
    })
}

/// Movies and episodes to continue: partially watched items plus the next
/// episode of shows whose latest watched episode was finished.
pub async fn continue_watching(db: &Db, limit: usize) -> Result<Vec<ContinueItem>> {
    let rows = sqlx::query_as::<_, PlayableItem>(&format!(
        "{PLAYABLE_SELECT}
         WHERE m.kind IN ('movie', 'episode') AND m.missing = 0 AND p.last_played_at IS NOT NULL
         ORDER BY p.last_played_at DESC LIMIT 200"
    ))
    .fetch_all(db.pool())
    .await?;

    let mut out = Vec::new();
    let mut seen_shows = HashSet::new();
    for item in rows {
        if out.len() >= limit {
            break;
        }
        let completed = item.completed.unwrap_or(false);
        let position = item.position.unwrap_or(0.0);
        let min_pos = item.duration.map(|d| (d * 0.05).min(MIN_RESUME_SECONDS)).unwrap_or(MIN_RESUME_SECONDS);
        if let Some(show_id) = item.show_id {
            if !seen_shows.insert(show_id) {
                continue;
            }
            if !completed && position >= min_pos {
                out.push(ContinueItem { item, reason: "resume".into() });
            } else if completed {
                let Some(episode_id) = item.episode_id else { continue };
                let current = video::episode(db, episode_id).await?;
                if let Some(next) = video::next_episode(db, &current).await? {
                    if next.completed != Some(true) {
                        let mut next_item = playable(db, next.media_id).await?;
                        next_item.last_played_at = item.last_played_at;
                        out.push(ContinueItem { item: next_item, reason: "next".into() });
                    }
                }
            }
        } else if !completed && position >= min_pos {
            out.push(ContinueItem { item, reason: "resume".into() });
        }
    }
    Ok(out)
}

/// Recently played items (one entry per media item, newest first).
pub async fn recently_played(db: &Db, kind: Option<&str>, offset: i64, limit: i64) -> Result<Page<PlayableItem>> {
    let limit = clamp_limit(limit);
    let kind_filter = match kind {
        Some("music") => "AND m.kind = 'track'",
        Some("video") => "AND m.kind IN ('movie', 'episode')",
        _ => "",
    };
    let items = sqlx::query_as::<_, PlayableItem>(&format!(
        "{PLAYABLE_SELECT} WHERE m.missing = 0 AND p.last_played_at IS NOT NULL AND p.play_count > 0 {kind_filter}
         ORDER BY p.last_played_at DESC LIMIT ? OFFSET ?"
    ))
    .bind(limit)
    .bind(offset)
    .fetch_all(db.pool())
    .await?;
    let total = sqlx::query_scalar::<_, i64>(&format!(
        "SELECT COUNT(*) FROM playback_progress p JOIN media m ON m.id = p.media_id
         WHERE m.missing = 0 AND p.play_count > 0 {kind_filter}"
    ))
    .fetch_one(db.pool())
    .await?;
    Ok(Page { items, total, offset, limit })
}

pub async fn clear_history(db: &Db) -> Result<()> {
    let mut tx = db.pool().begin().await?;
    sqlx::query("DELETE FROM play_history").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM playback_progress").execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
