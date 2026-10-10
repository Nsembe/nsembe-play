//! Global search.
//!
//! Candidates come from the FTS5 trigram index (any shared 3-character sequence
//! matches, which makes the query tolerant of typos), and are then re-ranked in
//! Rust with string-similarity scoring before being grouped by type.

use std::collections::{BTreeSet, HashMap};

use sqlx::FromRow;

use crate::database::{music, playlists, video, Db};
use crate::errors::Result;
use crate::models::SearchResults;

#[derive(Debug, FromRow)]
struct Candidate {
    kind: String,
    ref_id: i64,
    title: String,
    subtitle: String,
}

/// Minimum relevance (0..1) for a result to be shown.
const MIN_SCORE: f64 = 0.70;

pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

fn trigrams(tokens: &[String]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for token in tokens {
        let chars: Vec<char> = token.chars().collect();
        if chars.len() < 3 {
            continue;
        }
        for w in chars.windows(3) {
            out.insert(w.iter().collect());
        }
    }
    out
}

/// Similarity between one query token and one word of a title.
fn token_similarity(token: &str, word: &str) -> f64 {
    if word == token {
        return 1.0;
    }
    if word.starts_with(token) {
        // Prefix matches ("inter" -> "interstellar") are strong while typing.
        return 0.9 + 0.1 * (token.len() as f64 / word.len() as f64);
    }
    if token.chars().count() >= 4 && word.contains(token) {
        return 0.85;
    }
    let jw = strsim::jaro_winkler(token, word);
    let lev = strsim::normalized_damerau_levenshtein(token, word);
    jw.max(lev) * 0.95
}

/// Relevance of a candidate for the given query (0..1).
pub fn score(query: &str, tokens: &[String], title: &str, subtitle: &str) -> f64 {
    let title_l = title.to_lowercase();
    let q = query.trim().to_lowercase();
    if title_l == q {
        return 1.0;
    }
    if title_l.starts_with(&q) {
        return 0.97;
    }
    if q.chars().count() >= 3 && title_l.contains(&q) {
        return 0.9;
    }
    let title_words = tokenize(title);
    let sub_words = tokenize(subtitle);
    if tokens.is_empty() {
        return 0.0;
    }
    let mut total = 0.0;
    for token in tokens {
        let best_title = title_words.iter().map(|w| token_similarity(token, w)).fold(0.0, f64::max);
        let best_sub = sub_words.iter().map(|w| token_similarity(token, w) * 0.85).fold(0.0, f64::max);
        total += best_title.max(best_sub);
    }
    total / tokens.len() as f64
}

async fn candidates(db: &Db, query: &str, tokens: &[String]) -> Result<Vec<Candidate>> {
    let grams = trigrams(tokens);
    if !grams.is_empty() {
        let expr = grams.iter().map(|g| format!("\"{}\"", g.replace('"', ""))).collect::<Vec<_>>().join(" OR ");
        return Ok(sqlx::query_as::<_, Candidate>(
            "SELECT kind, ref_id, title, subtitle FROM search_index WHERE search_index MATCH ? ORDER BY rank LIMIT 600",
        )
        .bind(expr)
        .fetch_all(db.pool())
        .await?);
    }
    // Queries shorter than three characters: plain substring match.
    let like = format!("%{}%", query.trim().replace(['%', '_'], ""));
    Ok(sqlx::query_as::<_, Candidate>(
        "SELECT kind, ref_id, title, subtitle FROM search_index WHERE title LIKE ? LIMIT 300",
    )
    .bind(like)
    .fetch_all(db.pool())
    .await?)
}

/// Ranked result ids per kind.
pub async fn ranked_ids(db: &Db, query: &str, per_group: usize) -> Result<HashMap<String, Vec<i64>>> {
    let tokens = tokenize(query);
    if query.trim().is_empty() || tokens.is_empty() {
        return Ok(HashMap::new());
    }
    let mut scored: Vec<(f64, Candidate)> = candidates(db, query, &tokens)
        .await?
        .into_iter()
        .map(|c| (score(query, &tokens, &c.title, &c.subtitle), c))
        .filter(|(s, _)| *s >= MIN_SCORE)
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut groups: HashMap<String, Vec<i64>> = HashMap::new();
    for (_, c) in scored {
        let ids = groups.entry(c.kind).or_default();
        if ids.len() < per_group {
            ids.push(c.ref_id);
        }
    }
    Ok(groups)
}

pub async fn search(db: &Db, query: &str, per_group: usize) -> Result<SearchResults> {
    let per_group = per_group.clamp(1, 100);
    // Over-fetch ids: some may point at missing files and get filtered out.
    let groups = ranked_ids(db, query, per_group + 10).await?;
    let ids = |k: &str| groups.get(k).cloned().unwrap_or_default();
    fn take<T>(mut v: Vec<T>, n: usize) -> Vec<T> {
        v.truncate(n);
        v
    }

    Ok(SearchResults {
        query: query.to_string(),
        tracks: take(music::tracks_by_ids(db, &ids("track")).await?, per_group),
        albums: take(music::albums_by_ids(db, &ids("album")).await?, per_group),
        artists: take(music::artists_by_ids(db, &ids("artist")).await?, per_group),
        movies: take(video::movies_by_ids(db, &ids("movie")).await?, per_group),
        shows: take(video::shows_by_ids(db, &ids("tv_show")).await?, per_group),
        playlists: take(playlists::by_ids(db, &ids("playlist")).await?, per_group),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerates_typos() {
        let q = "interstelar";
        let t = tokenize(q);
        assert!(score(q, &t, "Interstellar", "2014") > MIN_SCORE);
        let q = "beyonse";
        let t = tokenize(q);
        assert!(score(q, &t, "Beyoncé", "") > MIN_SCORE);
    }

    #[test]
    fn rejects_unrelated() {
        let q = "inception";
        let t = tokenize(q);
        assert!(score(q, &t, "The Lion King", "1994 Animation") < MIN_SCORE);
    }

    #[test]
    fn prefers_prefix_matches() {
        let q = "dark";
        let t = tokenize(q);
        assert!(score(q, &t, "Dark", "") > score(q, &t, "The Dark Knight", ""));
        assert!(score(q, &t, "The Dark Knight", "") > MIN_SCORE);
    }
}
