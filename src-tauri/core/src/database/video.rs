//! Read queries for movies and TV shows.

use std::collections::BTreeSet;

use crate::database::music::reorder;
use crate::database::{clamp_limit, fav_expr, placeholders, Db};
use crate::errors::{CoreError, Result};
use crate::models::{Episode, Movie, MovieDetail, Page, Season, Show, ShowDetail, StreamInfo};

fn movie_select() -> String {
    format!(
        "SELECT mv.id, mv.media_id, mv.title, mv.year, mv.genres, mv.overview, m.duration,
                COALESCE(mv.poster_thumb, mv.poster_path) AS poster, mv.poster_path AS poster_full,
                COALESCE(mv.backdrop_path, m.thumb_path) AS backdrop,
                m.width, m.height, m.video_codec, m.audio_codec, m.container, m.added_at,
                p.position, p.completed, p.last_played_at, {fav} AS is_favorite
         FROM movies mv
         JOIN media m ON m.id = mv.media_id
         LEFT JOIN playback_progress p ON p.media_id = m.id
         WHERE m.missing = 0",
        fav = fav_expr("movie", "mv.id")
    )
}

fn show_select() -> String {
    format!(
        "SELECT s.id, s.title, s.year, s.overview, s.genres,
                COALESCE(s.poster_thumb, s.poster_path) AS poster, s.poster_path AS poster_full,
                COALESCE(s.backdrop_path,
                    (SELECT m.thumb_path FROM tv_episodes e JOIN media m ON m.id = e.media_id
                     WHERE e.show_id = s.id AND m.thumb_path IS NOT NULL
                     ORDER BY e.season_number, e.episode_number LIMIT 1)) AS backdrop,
                (SELECT COUNT(DISTINCT e.season_number) FROM tv_episodes e JOIN media m ON m.id = e.media_id
                    WHERE e.show_id = s.id AND m.missing = 0) AS season_count,
                (SELECT COUNT(*) FROM tv_episodes e JOIN media m ON m.id = e.media_id
                    WHERE e.show_id = s.id AND m.missing = 0) AS episode_count,
                (SELECT COUNT(*) FROM tv_episodes e JOIN media m ON m.id = e.media_id
                    JOIN playback_progress p ON p.media_id = e.media_id
                    WHERE e.show_id = s.id AND m.missing = 0 AND p.completed = 1) AS watched_count,
                COALESCE((SELECT MAX(m.added_at) FROM tv_episodes e JOIN media m ON m.id = e.media_id
                    WHERE e.show_id = s.id), s.added_at) AS added_at,
                {fav} AS is_favorite
         FROM tv_shows s
         WHERE EXISTS (SELECT 1 FROM tv_episodes e JOIN media m ON m.id = e.media_id
                       WHERE e.show_id = s.id AND m.missing = 0)",
        fav = fav_expr("tv_show", "s.id")
    )
}

const EPISODE_SELECT: &str = "
    SELECT e.id, e.media_id, e.show_id, s.title AS show_title, e.season_number, e.episode_number, e.title,
           e.overview, m.duration, m.thumb_path AS thumb, p.position, p.completed
    FROM tv_episodes e
    JOIN tv_shows s ON s.id = e.show_id
    JOIN media m ON m.id = e.media_id
    LEFT JOIN playback_progress p ON p.media_id = e.media_id
    WHERE m.missing = 0";

fn movie_order(sort: &str) -> &'static str {
    match sort {
        "added" => "m.added_at DESC, mv.id DESC",
        "year" => "mv.year DESC, mv.sort_title COLLATE NOCASE",
        "duration" => "m.duration DESC",
        "recent" => "p.last_played_at DESC NULLS LAST, m.added_at DESC",
        _ => "mv.sort_title COLLATE NOCASE, mv.year",
    }
}

fn genre_clause() -> &'static str {
    // genres are stored comma separated without padding: "Action,Sci-Fi"
    "(',' || COALESCE(mv.genres, '') || ',') LIKE ('%,' || ? || ',%')"
}

pub async fn list_movies(db: &Db, sort: &str, genre: Option<&str>, offset: i64, limit: i64) -> Result<Page<Movie>> {
    let limit = clamp_limit(limit);
    let filter = if genre.is_some() { format!("AND {}", genre_clause()) } else { String::new() };
    let sql = format!("{} {filter} ORDER BY {} LIMIT ? OFFSET ?", movie_select(), movie_order(sort));
    let mut q = sqlx::query_as::<_, Movie>(&sql);
    if let Some(g) = genre {
        q = q.bind(g);
    }
    let items = q.bind(limit).bind(offset).fetch_all(db.pool()).await?;
    let count_sql = format!(
        "SELECT COUNT(*) FROM movies mv JOIN media m ON m.id = mv.media_id WHERE m.missing = 0 {filter}"
    );
    let mut cq = sqlx::query_scalar::<_, i64>(&count_sql);
    if let Some(g) = genre {
        cq = cq.bind(g);
    }
    let total = cq.fetch_one(db.pool()).await?;
    Ok(Page { items, total, offset, limit })
}

pub async fn movies_by_ids(db: &Db, ids: &[i64]) -> Result<Vec<Movie>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let sql = format!("{} AND mv.id IN ({})", movie_select(), placeholders(ids.len()));
    let mut q = sqlx::query_as::<_, Movie>(&sql);
    for id in ids {
        q = q.bind(id);
    }
    Ok(reorder(q.fetch_all(db.pool()).await?, ids, |m| m.id))
}

pub async fn movie(db: &Db, id: i64) -> Result<Movie> {
    movies_by_ids(db, &[id]).await?.into_iter().next().ok_or_else(|| CoreError::not_found("Movie not found"))
}

pub async fn movie_detail(db: &Db, id: i64) -> Result<MovieDetail> {
    let movie = movie(db, id).await?;
    let (path, file_size, audio, subs) = sqlx::query_as::<_, (String, i64, Option<String>, Option<String>)>(
        "SELECT path, file_size, audio_streams, subtitle_streams FROM media WHERE id = ?",
    )
    .bind(movie.media_id)
    .fetch_one(db.pool())
    .await?;
    let parse = |s: Option<String>| -> Vec<StreamInfo> {
        s.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    };
    let more_like_this = more_like_this(db, &movie, 18).await?;
    Ok(MovieDetail {
        movie,
        path,
        file_size,
        audio_streams: parse(audio),
        subtitle_streams: parse(subs),
        more_like_this,
    })
}

/// Local recommendations: shared genres first, then closeness in release year.
pub async fn more_like_this(db: &Db, movie: &Movie, limit: i64) -> Result<Vec<Movie>> {
    let genres: Vec<String> = movie
        .genres
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|g| g.trim().to_string())
        .filter(|g| !g.is_empty())
        .take(6)
        .collect();
    let score = if genres.is_empty() {
        "0".to_string()
    } else {
        genres.iter().map(|_| format!("(CASE WHEN {} THEN 1 ELSE 0 END)", genre_clause())).collect::<Vec<_>>().join(" + ")
    };
    let sql = format!(
        "SELECT * FROM ({base} AND mv.id <> ?) AS c
         ORDER BY ({score_outer}) DESC, ABS(COALESCE(c.year, 0) - ?) ASC, c.added_at DESC
         LIMIT ?",
        base = movie_select().replacen(
            "SELECT mv.id,",
            &format!("SELECT ({score}) AS genre_score, mv.id,"),
            1
        ),
        score_outer = "c.genre_score"
    );
    let mut q = sqlx::query_as::<_, Movie>(&sql);
    for g in &genres {
        q = q.bind(g);
    }
    Ok(q.bind(movie.id).bind(movie.year.unwrap_or(0)).bind(limit).fetch_all(db.pool()).await?)
}

pub async fn movie_genres(db: &Db) -> Result<Vec<String>> {
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT mv.genres FROM movies mv JOIN media m ON m.id = mv.media_id
         WHERE m.missing = 0 AND mv.genres IS NOT NULL AND mv.genres <> ''",
    )
    .fetch_all(db.pool())
    .await?;
    let set: BTreeSet<String> = rows
        .iter()
        .flat_map(|g| g.split(','))
        .map(|g| g.trim().to_string())
        .filter(|g| !g.is_empty())
        .collect();
    Ok(set.into_iter().collect())
}

pub async fn list_shows(db: &Db, sort: &str, offset: i64, limit: i64) -> Result<Page<Show>> {
    let limit = clamp_limit(limit);
    let order = match sort {
        "added" => "added_at DESC",
        "year" => "s.year DESC, s.title COLLATE NOCASE",
        _ => "s.title COLLATE NOCASE",
    };
    let sql = format!("{} ORDER BY {order} LIMIT ? OFFSET ?", show_select());
    let items = sqlx::query_as::<_, Show>(&sql).bind(limit).bind(offset).fetch_all(db.pool()).await?;
    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tv_shows s WHERE EXISTS (SELECT 1 FROM tv_episodes e JOIN media m ON m.id = e.media_id
            WHERE e.show_id = s.id AND m.missing = 0)",
    )
    .fetch_one(db.pool())
    .await?;
    Ok(Page { items, total, offset, limit })
}

pub async fn shows_by_ids(db: &Db, ids: &[i64]) -> Result<Vec<Show>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let sql = format!("{} AND s.id IN ({})", show_select(), placeholders(ids.len()));
    let mut q = sqlx::query_as::<_, Show>(&sql);
    for id in ids {
        q = q.bind(id);
    }
    Ok(reorder(q.fetch_all(db.pool()).await?, ids, |s| s.id))
}

pub async fn show_detail(db: &Db, show_id: i64) -> Result<ShowDetail> {
    let show = shows_by_ids(db, &[show_id])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| CoreError::not_found("TV show not found"))?;
    let seasons = sqlx::query_as::<_, Season>(
        "SELECT se.id, se.show_id, se.season_number,
                COALESCE(se.poster_path, s.poster_thumb, s.poster_path) AS poster,
                (SELECT COUNT(*) FROM tv_episodes e JOIN media m ON m.id = e.media_id
                    WHERE e.season_id = se.id AND m.missing = 0) AS episode_count
         FROM tv_seasons se JOIN tv_shows s ON s.id = se.show_id
         WHERE se.show_id = ? AND EXISTS (SELECT 1 FROM tv_episodes e JOIN media m ON m.id = e.media_id
                    WHERE e.season_id = se.id AND m.missing = 0)
         ORDER BY CASE WHEN se.season_number = 0 THEN 9999 ELSE se.season_number END",
    )
    .bind(show_id)
    .fetch_all(db.pool())
    .await?;
    let episodes = sqlx::query_as::<_, Episode>(&format!(
        "{EPISODE_SELECT} AND e.show_id = ? ORDER BY e.season_number, e.episode_number"
    ))
    .bind(show_id)
    .fetch_all(db.pool())
    .await?;
    let next_up = show_next_up(db, show_id).await?;
    Ok(ShowDetail { show, seasons, episodes, next_up })
}

pub async fn episode(db: &Db, episode_id: i64) -> Result<Episode> {
    sqlx::query_as::<_, Episode>(&format!("{EPISODE_SELECT} AND e.id = ?"))
        .bind(episode_id)
        .fetch_optional(db.pool())
        .await?
        .ok_or_else(|| CoreError::not_found("Episode not found"))
}

pub async fn episode_by_media(db: &Db, media_id: i64) -> Result<Option<Episode>> {
    Ok(sqlx::query_as::<_, Episode>(&format!("{EPISODE_SELECT} AND e.media_id = ?"))
        .bind(media_id)
        .fetch_optional(db.pool())
        .await?)
}

/// The episode after `current` in broadcast order. Specials (season 0) are only
/// followed by other specials.
pub async fn next_episode(db: &Db, current: &Episode) -> Result<Option<Episode>> {
    let specials = if current.season_number > 0 { "AND e.season_number > 0" } else { "" };
    Ok(sqlx::query_as::<_, Episode>(&format!(
        "{EPISODE_SELECT} AND e.show_id = ? {specials}
           AND (e.season_number > ? OR (e.season_number = ? AND e.episode_number > ?))
         ORDER BY e.season_number, e.episode_number LIMIT 1"
    ))
    .bind(current.show_id)
    .bind(current.season_number)
    .bind(current.season_number)
    .bind(current.episode_number)
    .fetch_optional(db.pool())
    .await?)
}

pub async fn previous_episode(db: &Db, current: &Episode) -> Result<Option<Episode>> {
    let specials = if current.season_number > 0 { "AND e.season_number > 0" } else { "" };
    Ok(sqlx::query_as::<_, Episode>(&format!(
        "{EPISODE_SELECT} AND e.show_id = ? {specials}
           AND (e.season_number < ? OR (e.season_number = ? AND e.episode_number < ?))
         ORDER BY e.season_number DESC, e.episode_number DESC LIMIT 1"
    ))
    .bind(current.show_id)
    .bind(current.season_number)
    .bind(current.season_number)
    .bind(current.episode_number)
    .fetch_optional(db.pool())
    .await?)
}

/// What "Play" on a show should start: the in-progress episode, the episode after
/// the last finished one, or the very first episode.
pub async fn show_next_up(db: &Db, show_id: i64) -> Result<Option<Episode>> {
    let last = sqlx::query_as::<_, Episode>(&format!(
        "{EPISODE_SELECT} AND e.show_id = ? AND p.last_played_at IS NOT NULL
         ORDER BY (SELECT p2.last_played_at FROM playback_progress p2 WHERE p2.media_id = e.media_id) DESC LIMIT 1"
    ))
    .bind(show_id)
    .fetch_optional(db.pool())
    .await?;
    if let Some(ep) = last {
        if ep.completed != Some(true) {
            return Ok(Some(ep));
        }
        if let Some(next) = next_episode(db, &ep).await? {
            return Ok(Some(next));
        }
    }
    Ok(sqlx::query_as::<_, Episode>(&format!(
        "{EPISODE_SELECT} AND e.show_id = ?
         ORDER BY CASE WHEN e.season_number = 0 THEN 9999 ELSE e.season_number END, e.episode_number LIMIT 1"
    ))
    .bind(show_id)
    .fetch_optional(db.pool())
    .await?)
}
