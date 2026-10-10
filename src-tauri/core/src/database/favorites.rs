//! Favorites for songs, albums, artists, movies and TV shows.

use crate::database::{music, video, Db};
use crate::errors::{CoreError, Result};
use crate::models::FavoritesOverview;
use crate::util::now;

pub const FAVORITE_TYPES: [&str; 5] = ["track", "album", "artist", "movie", "tv_show"];

fn validate(item_type: &str) -> Result<()> {
    if FAVORITE_TYPES.contains(&item_type) {
        Ok(())
    } else {
        Err(CoreError::invalid(format!("'{item_type}' cannot be a favorite")))
    }
}

pub async fn is_favorite(db: &Db, item_type: &str, item_id: i64) -> Result<bool> {
    let n = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM favorites WHERE item_type = ? AND item_id = ?")
        .bind(item_type)
        .bind(item_id)
        .fetch_one(db.pool())
        .await?;
    Ok(n > 0)
}

pub async fn set(db: &Db, item_type: &str, item_id: i64, favorite: bool) -> Result<bool> {
    validate(item_type)?;
    if favorite {
        sqlx::query("INSERT OR IGNORE INTO favorites(item_type, item_id, created_at) VALUES (?, ?, ?)")
            .bind(item_type)
            .bind(item_id)
            .bind(now())
            .execute(db.pool())
            .await?;
    } else {
        sqlx::query("DELETE FROM favorites WHERE item_type = ? AND item_id = ?")
            .bind(item_type)
            .bind(item_id)
            .execute(db.pool())
            .await?;
    }
    Ok(favorite)
}

pub async fn toggle(db: &Db, item_type: &str, item_id: i64) -> Result<bool> {
    validate(item_type)?;
    let current = is_favorite(db, item_type, item_id).await?;
    set(db, item_type, item_id, !current).await
}

async fn ids(db: &Db, item_type: &str) -> Result<Vec<i64>> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT item_id FROM favorites WHERE item_type = ? ORDER BY created_at DESC LIMIT 500",
    )
    .bind(item_type)
    .fetch_all(db.pool())
    .await?)
}

pub async fn overview(db: &Db) -> Result<FavoritesOverview> {
    Ok(FavoritesOverview {
        tracks: music::tracks_by_ids(db, &ids(db, "track").await?).await?,
        albums: music::albums_by_ids(db, &ids(db, "album").await?).await?,
        artists: music::artists_by_ids(db, &ids(db, "artist").await?).await?,
        movies: video::movies_by_ids(db, &ids(db, "movie").await?).await?,
        shows: video::shows_by_ids(db, &ids(db, "tv_show").await?).await?,
    })
}
