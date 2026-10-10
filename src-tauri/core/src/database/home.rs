//! Home screen feed and library-wide listings.

use crate::database::{clamp_limit, music, playback, playlists, video, Db};
use crate::errors::Result;
use crate::models::{HomeFeed, LibraryStats, Page, RecentItem};

async fn count(db: &Db, sql: &str) -> Result<i64> {
    Ok(sqlx::query_scalar::<_, i64>(sql).fetch_one(db.pool()).await?)
}

pub async fn stats(db: &Db) -> Result<LibraryStats> {
    Ok(LibraryStats {
        tracks: count(db, "SELECT COUNT(*) FROM tracks t JOIN media m ON m.id = t.media_id WHERE m.missing = 0").await?,
        albums: count(
            db,
            "SELECT COUNT(*) FROM albums a WHERE EXISTS (SELECT 1 FROM tracks t JOIN media m ON m.id = t.media_id
                WHERE t.album_id = a.id AND m.missing = 0)",
        )
        .await?,
        artists: count(
            db,
            "SELECT COUNT(*) FROM artists ar WHERE EXISTS (SELECT 1 FROM tracks t JOIN media m ON m.id = t.media_id
                WHERE t.artist_id = ar.id AND m.missing = 0)",
        )
        .await?,
        movies: count(db, "SELECT COUNT(*) FROM movies mv JOIN media m ON m.id = mv.media_id WHERE m.missing = 0")
            .await?,
        shows: count(
            db,
            "SELECT COUNT(*) FROM tv_shows s WHERE EXISTS (SELECT 1 FROM tv_episodes e JOIN media m ON m.id = e.media_id
                WHERE e.show_id = s.id AND m.missing = 0)",
        )
        .await?,
        episodes: count(db, "SELECT COUNT(*) FROM tv_episodes e JOIN media m ON m.id = e.media_id WHERE m.missing = 0")
            .await?,
    })
}

const RECENT_UNION: &str = "
    SELECT 'album' AS kind, a.id AS id, a.title AS title, ar.name AS subtitle, a.artwork_thumb AS artwork,
           a.added_at AS added_at
    FROM albums a JOIN artists ar ON ar.id = a.artist_id
    WHERE EXISTS (SELECT 1 FROM tracks t JOIN media m ON m.id = t.media_id WHERE t.album_id = a.id AND m.missing = 0)
    UNION ALL
    SELECT 'movie', mv.id, mv.title, CAST(mv.year AS TEXT),
           COALESCE(mv.poster_thumb, mv.poster_path, mv.backdrop_path, m.thumb_path), m.added_at
    FROM movies mv JOIN media m ON m.id = mv.media_id WHERE m.missing = 0
    UNION ALL
    SELECT 'tv_show', s.id, s.title,
           (SELECT COUNT(*) FROM tv_episodes e JOIN media m ON m.id = e.media_id
               WHERE e.show_id = s.id AND m.missing = 0) || ' episodes',
           COALESCE(s.poster_thumb, s.poster_path, s.backdrop_path),
           (SELECT MAX(m.added_at) FROM tv_episodes e JOIN media m ON m.id = e.media_id
               WHERE e.show_id = s.id AND m.missing = 0)
    FROM tv_shows s
    WHERE EXISTS (SELECT 1 FROM tv_episodes e JOIN media m ON m.id = e.media_id WHERE e.show_id = s.id AND m.missing = 0)";

pub async fn recently_added(db: &Db, offset: i64, limit: i64) -> Result<Page<RecentItem>> {
    let limit = clamp_limit(limit);
    let items = sqlx::query_as::<_, RecentItem>(&format!(
        "SELECT * FROM ({RECENT_UNION}) ORDER BY added_at DESC, id DESC LIMIT ? OFFSET ?"
    ))
    .bind(limit)
    .bind(offset)
    .fetch_all(db.pool())
    .await?;
    let total = count(db, &format!("SELECT COUNT(*) FROM ({RECENT_UNION})")).await?;
    Ok(Page { items, total, offset, limit })
}

pub async fn feed(db: &Db) -> Result<HomeFeed> {
    Ok(HomeFeed {
        continue_watching: playback::continue_watching(db, 20).await?,
        recently_added: recently_added(db, 0, 20).await?.items,
        recently_played: playback::recently_played(db, None, 0, 20).await?.items,
        playlists: playlists::list(db).await?.into_iter().take(20).collect(),
        albums: music::list_albums(db, "added", 0, 20).await?.items,
        movies: video::list_movies(db, "added", None, 0, 20).await?.items,
        shows: video::list_shows(db, "added", 0, 20).await?.items,
        stats: stats(db).await?,
    })
}
