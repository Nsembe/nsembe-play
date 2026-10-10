//! Filename/folder parsing for media that lacks embedded metadata.
//!
//! Handles common naming conventions:
//! - Movies: `Inception (2010).mkv`, `Inception.2010.1080p.BluRay.x264.mkv`,
//!   `Inception (2010)/movie.mkv`
//! - TV: `Show/Season 01/S01E02 - Title.mkv`, `Show.Name.S01E02.720p.mkv`, `1x02`
//! - Music: `01 - Title.mp3`, `01 - Artist - Title.flac`

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

static YEAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|[\s\(\[\-])((?:19|20)\d{2})(?:$|[\s\)\]\-])").unwrap());

static RELEASE_TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:^|\s)(?:2160p|1080p|1080i|720p|576p|480p|4k|uhd|hdr10?|dolby|dv|bluray|blu-ray|bdrip|brrip|bdremux|remux|web-?dl|webrip|web|hdtv|dvdrip|dvdscr|hdrip|xvid|divx|x264|x265|h\.?264|h\.?265|hevc|avc|aac\d?(?:\.\d)?|ac3|dts(?:-hd)?|truehd|atmos|10bit|8bit|proper|repack|extended|unrated|remastered|imax|multi|dual|subbed|dubbed|internal|limited)(?:$|\s)",
    )
    .unwrap()
});

static SXXEYY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bs(\d{1,2})[\s._-]*e(\d{1,3})(?:[\s._-]*e\d{1,3})*").unwrap());
static NXNN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\b(\d{1,2})x(\d{2,3})\b").unwrap());
static SEASON_EPISODE_WORDS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)season[\s._-]*(\d{1,2})[\s._-]*episode[\s._-]*(\d{1,3})").unwrap());
static EPISODE_ONLY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:^|[\s._-])(?:e|ep|episode)[\s._-]*(\d{1,3})(?:$|[\s._-])").unwrap());
static LEADING_NUMBER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d{1,3})(?:[\s._-]+(.*))?$").unwrap());
static SEASON_DIR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?:season|series|staffel|saison|s)[\s._-]*(\d{1,3})$").unwrap());
static MULTI_SPACE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
static BRACKETS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[[^\]]*\]|\{[^}]*\}").unwrap());

/// Replaces scene-style separators with spaces and tidies whitespace.
/// Dots are kept when the name already uses spaces ("Mr. Robot").
pub fn clean_name(raw: &str) -> String {
    let without_brackets = BRACKETS_RE.replace_all(raw, " ");
    let mut s = without_brackets.replace('_', " ");
    let has_spaces = s.trim().contains(' ');
    if !has_spaces || s.matches('.').count() >= 2 {
        s = s.replace('.', " ");
    }
    let s = MULTI_SPACE_RE.replace_all(&s, " ");
    s.trim_matches(|c: char| c.is_whitespace() || c == '-' || c == '(' || c == '[' || c == ',')
        .trim()
        .to_string()
}

fn cut_release_tags(s: &str) -> String {
    match RELEASE_TAG_RE.find(s) {
        Some(m) if m.start() > 0 => s[..m.start()].to_string(),
        _ => s.to_string(),
    }
}

fn trim_title(s: &str) -> String {
    let s = s.trim().trim_end_matches(|c: char| c == '(' || c == '[' || c == '-' || c == '.' || c.is_whitespace());
    let s = s.trim_start_matches(|c: char| c == '-' || c == '.' || c.is_whitespace());
    // Unbalanced trailing ")" from "(2010)" removal.
    let s = if s.ends_with(')') && !s.contains('(') { &s[..s.len() - 1] } else { s };
    s.trim().to_string()
}

/// Parses a movie title and year from a file stem.
pub fn parse_movie_name(stem: &str) -> (String, Option<i64>) {
    let cleaned = clean_name(stem);
    // Use the last year that is not at the very start ("2001 A Space Odyssey (1968)").
    let year_match = YEAR_RE.captures_iter(&cleaned).filter_map(|c| c.get(1)).filter(|m| m.start() > 0).last();
    if let Some(m) = year_match {
        let title = trim_title(&cleaned[..m.start()]);
        if !title.is_empty() {
            return (title, m.as_str().parse().ok());
        }
    }
    let title = trim_title(&cut_release_tags(&cleaned));
    if title.is_empty() {
        (trim_title(&cleaned), None)
    } else {
        (title, None)
    }
}

/// Picks the best title source for a movie file: the file name, or its dedicated
/// parent folder when the file name is generic or lacks a year.
pub fn parse_movie_path(path: &Path, library_root: &Path) -> (String, Option<i64>) {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    let (title, year) = parse_movie_name(stem);
    let parent = path.parent().filter(|p| *p != library_root && p.starts_with(library_root));
    if let Some(dir_name) = parent.and_then(|p| p.file_name()).and_then(|n| n.to_str()) {
        let (dir_title, dir_year) = parse_movie_name(dir_name);
        let generic = title.len() <= 3 || ["movie", "film", "video", "feature"].contains(&title.to_lowercase().as_str());
        if dir_year.is_some() && (year.is_none() || generic) {
            return (dir_title, dir_year);
        }
        if generic && !dir_title.is_empty() {
            return (dir_title, dir_year.or(year));
        }
    }
    (title, year)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeInfo {
    pub show: String,
    pub show_year: Option<i64>,
    pub show_folder: Option<String>,
    pub season: i64,
    pub season_folder: Option<String>,
    pub episode: i64,
    pub title: String,
}

/// Season number from a directory name like "Season 01", "S2" or "Specials".
pub fn season_from_dir(name: &str) -> Option<i64> {
    let name = name.trim();
    if name.eq_ignore_ascii_case("specials") || name.eq_ignore_ascii_case("extras") {
        return Some(0);
    }
    SEASON_DIR_RE.captures(name).and_then(|c| c[1].parse().ok())
}

fn episode_title(rest: &str, episode: i64) -> String {
    let t = trim_title(&cut_release_tags(&clean_name(rest)));
    if t.is_empty() || t.chars().all(|c| c.is_ascii_digit()) {
        format!("Episode {episode}")
    } else {
        t
    }
}

/// Parses TV episode information from a path relative to a TV library root.
pub fn parse_episode_path(path: &Path, library_root: &Path) -> EpisodeInfo {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    let rel = path.strip_prefix(library_root).unwrap_or(path);
    let dirs: Vec<&str> =
        rel.parent().map(|p| p.iter().filter_map(|c| c.to_str()).collect()).unwrap_or_default();

    // Show folder = first directory below the root; season folder = a later directory named like a season.
    let mut show_folder: Option<(String, String)> = None; // (name, full path)
    let mut season_folder: Option<(i64, String)> = None;
    let mut acc = library_root.to_path_buf();
    for (i, dir) in dirs.iter().enumerate() {
        acc.push(dir);
        if let Some(n) = season_from_dir(dir) {
            season_folder = Some((n, acc.to_string_lossy().into_owned()));
        } else if i == 0 {
            show_folder = Some((dir.to_string(), acc.to_string_lossy().into_owned()));
        }
    }

    let cleaned = clean_name(stem);
    let mut season: Option<i64> = None;
    let mut episode: Option<i64> = None;
    let mut before = String::new();
    let mut after = String::new();

    for re in [&*SXXEYY_RE, &*NXNN_RE, &*SEASON_EPISODE_WORDS_RE] {
        if let Some(c) = re.captures(&cleaned) {
            let whole = c.get(0).unwrap();
            season = c[1].parse().ok();
            episode = c[2].parse().ok();
            before = cleaned[..whole.start()].to_string();
            after = cleaned[whole.end()..].to_string();
            break;
        }
    }
    if episode.is_none() {
        if let Some(c) = EPISODE_ONLY_RE.captures(&cleaned) {
            let whole = c.get(0).unwrap();
            episode = c[1].parse().ok();
            before = cleaned[..whole.start()].to_string();
            after = cleaned[whole.end()..].to_string();
        } else if let Some(c) = LEADING_NUMBER_RE.captures(&cleaned) {
            episode = c[1].parse().ok();
            after = c.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
        }
    }

    let season = season.or(season_folder.as_ref().map(|s| s.0)).unwrap_or(1);
    let episode = episode.unwrap_or(0);

    let (show, show_year, show_path) = match show_folder {
        Some((name, full)) => {
            let (t, y) = parse_movie_name(&name);
            (t, y, Some(full))
        }
        None => {
            let from_name = trim_title(&before);
            let (t, y) = parse_movie_name(&from_name);
            let t = if t.is_empty() { "Unknown Show".to_string() } else { t };
            (t, y, None)
        }
    };

    EpisodeInfo {
        show,
        show_year,
        show_folder: show_path,
        season,
        season_folder: season_folder.map(|s| s.1),
        episode,
        title: episode_title(&after, episode),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrackNameInfo {
    pub track_number: Option<i64>,
    pub artist: Option<String>,
    pub title: String,
}

/// Parses "01 - Title", "01. Artist - Title" or "Artist - Title".
pub fn parse_track_name(stem: &str) -> TrackNameInfo {
    let s = stem.replace('_', " ");
    let s = s.trim();
    let (track_number, rest) = match LEADING_NUMBER_RE.captures(s) {
        Some(c) if c.get(2).is_some() => (c[1].parse().ok(), c.get(2).unwrap().as_str().to_string()),
        _ => (None, s.to_string()),
    };
    let rest = rest.trim_start_matches(|c: char| c == '.' || c == '-' || c.is_whitespace()).to_string();
    if let Some((artist, title)) = rest.split_once(" - ") {
        if !artist.trim().is_empty() && !title.trim().is_empty() {
            return TrackNameInfo {
                track_number,
                artist: Some(artist.trim().to_string()),
                title: title.trim().to_string(),
            };
        }
    }
    TrackNameInfo { track_number, artist: None, title: if rest.is_empty() { s.to_string() } else { rest } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn movie_with_year_in_parentheses() {
        assert_eq!(parse_movie_name("Inception (2010)"), ("Inception".into(), Some(2010)));
    }

    #[test]
    fn scene_style_movie() {
        assert_eq!(parse_movie_name("The.Dark.Knight.2008.1080p.BluRay.x264-GROUP"), ("The Dark Knight".into(), Some(2008)));
    }

    #[test]
    fn movie_starting_with_number() {
        assert_eq!(parse_movie_name("2001 A Space Odyssey (1968)"), ("2001 A Space Odyssey".into(), Some(1968)));
        assert_eq!(parse_movie_name("1917 (2019)"), ("1917".into(), Some(2019)));
    }

    #[test]
    fn movie_without_year() {
        assert_eq!(parse_movie_name("Amelie.1080p.WEB-DL"), ("Amelie".into(), None));
        assert_eq!(parse_movie_name("Mr. Nobody"), ("Mr. Nobody".into(), None));
    }

    #[test]
    fn movie_uses_folder_name() {
        let root = PathBuf::from("/media/movies");
        let p = root.join("Arrival (2016)").join("arrival-1080p.mkv");
        assert_eq!(parse_movie_path(&p, &root), ("Arrival".into(), Some(2016)));
        let p = root.join("Heat (1995).mkv");
        assert_eq!(parse_movie_path(&p, &root), ("Heat".into(), Some(1995)));
    }

    #[test]
    fn episode_in_season_folder() {
        let root = PathBuf::from("/tv");
        let p = root.join("Breaking Bad").join("Season 02").join("S02E05 - Breakage.mkv");
        let e = parse_episode_path(&p, &root);
        assert_eq!(e.show, "Breaking Bad");
        assert_eq!(e.season, 2);
        assert_eq!(e.episode, 5);
        assert_eq!(e.title, "Breakage");
    }

    #[test]
    fn scene_style_episode_without_folders() {
        let root = PathBuf::from("/tv");
        let p = root.join("The.Office.US.S03E10.A.Benihana.Christmas.720p.HDTV.x264.mkv");
        let e = parse_episode_path(&p, &root);
        assert_eq!(e.show, "The Office US");
        assert_eq!((e.season, e.episode), (3, 10));
        assert_eq!(e.title, "A Benihana Christmas");
    }

    #[test]
    fn nxnn_and_episode_only() {
        let root = PathBuf::from("/tv");
        let e = parse_episode_path(&root.join("Firefly").join("1x07 Jaynestown.avi"), &root);
        assert_eq!((e.show.as_str(), e.season, e.episode), ("Firefly", 1, 7));
        let e = parse_episode_path(&root.join("Dark").join("Season 3").join("Episode 4.mkv"), &root);
        assert_eq!((e.season, e.episode), (3, 4));
        assert_eq!(e.title, "Episode 4");
        let e = parse_episode_path(&root.join("Show").join("S1").join("03 - Title.mkv"), &root);
        assert_eq!((e.season, e.episode, e.title.as_str()), (1, 3, "Title"));
    }

    #[test]
    fn plain_spec_layout() {
        let root = PathBuf::from("/tv");
        let e = parse_episode_path(&root.join("Show Name").join("Season 01").join("S01E01.mkv"), &root);
        assert_eq!((e.show.as_str(), e.season, e.episode, e.title.as_str()), ("Show Name", 1, 1, "Episode 1"));
        assert!(e.season_folder.is_some());
    }

    #[test]
    fn specials_folder() {
        assert_eq!(season_from_dir("Specials"), Some(0));
        assert_eq!(season_from_dir("Season 10"), Some(10));
        assert_eq!(season_from_dir("Behind the scenes"), None);
    }

    #[test]
    fn track_names() {
        assert_eq!(
            parse_track_name("01 - Speak to Me"),
            TrackNameInfo { track_number: Some(1), artist: None, title: "Speak to Me".into() }
        );
        assert_eq!(
            parse_track_name("07. Radiohead - Reckoner"),
            TrackNameInfo { track_number: Some(7), artist: Some("Radiohead".into()), title: "Reckoner".into() }
        );
        assert_eq!(parse_track_name("Untitled").title, "Untitled");
    }
}
