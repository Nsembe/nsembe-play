//! Minimal reader for Kodi-style `.nfo` XML files (title, year, plot, genres).

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NfoInfo {
    pub title: Option<String>,
    pub year: Option<i64>,
    pub plot: Option<String>,
    pub genres: Vec<String>,
}

static TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<(title|year|plot|outline|genre|premiered)>(.*?)</(?:title|year|plot|outline|genre|premiered)>")
        .unwrap()
});

fn unescape(s: &str) -> String {
    let s = s.trim();
    let s = s.strip_prefix("<![CDATA[").and_then(|x| x.strip_suffix("]]>")).unwrap_or(s);
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .trim()
        .to_string()
}

pub fn parse_nfo(content: &str) -> Option<NfoInfo> {
    if !content.contains('<') {
        return None; // URL-only nfo files
    }
    let mut info = NfoInfo::default();
    for cap in TAG_RE.captures_iter(content) {
        let value = unescape(&cap[2]);
        if value.is_empty() {
            continue;
        }
        match &cap[1] {
            // The first <title> is the item's own title (actors etc. come later).
            "title" if info.title.is_none() => info.title = Some(value),
            "year" if info.year.is_none() => info.year = value.parse().ok(),
            "premiered" if info.year.is_none() => info.year = value.get(..4).and_then(|y| y.parse().ok()),
            "plot" => info.plot = Some(value),
            "outline" if info.plot.is_none() => info.plot = Some(value),
            "genre" => {
                for g in value.split(['/', ',']) {
                    let g = g.trim().to_string();
                    if !g.is_empty() && !info.genres.contains(&g) {
                        info.genres.push(g);
                    }
                }
            }
            _ => {}
        }
    }
    if info == NfoInfo::default() {
        None
    } else {
        Some(info)
    }
}

fn read_nfo(path: &Path) -> Option<NfoInfo> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() > 2 * 1024 * 1024 {
        return None;
    }
    parse_nfo(&String::from_utf8_lossy(&bytes))
}

/// Looks for `<stem>.nfo` or `movie.nfo` beside a movie file.
pub fn find_movie_nfo(media_path: &Path) -> Option<NfoInfo> {
    let dir = media_path.parent()?;
    [media_path.with_extension("nfo"), dir.join("movie.nfo")]
        .into_iter()
        .filter(|p| p.is_file())
        .find_map(|p| read_nfo(&p))
}

/// `tvshow.nfo` in a show folder.
pub fn find_show_nfo(show_dir: &Path) -> Option<NfoInfo> {
    let p = show_dir.join("tvshow.nfo");
    if p.is_file() {
        read_nfo(&p)
    } else {
        None
    }
}

/// `<stem>.nfo` beside an episode file.
pub fn find_episode_nfo(media_path: &Path) -> Option<NfoInfo> {
    let p = media_path.with_extension("nfo");
    if p.is_file() {
        read_nfo(&p)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kodi_movie_nfo() {
        let xml = r#"<?xml version="1.0"?>
<movie>
  <title>Arrival</title>
  <year>2016</year>
  <plot>A linguist works with the military to communicate with alien lifeforms.</plot>
  <genre>Drama</genre>
  <genre>Science Fiction / Mystery</genre>
  <actor><name>Amy Adams</name></actor>
</movie>"#;
        let n = parse_nfo(xml).unwrap();
        assert_eq!(n.title.as_deref(), Some("Arrival"));
        assert_eq!(n.year, Some(2016));
        assert_eq!(n.genres, vec!["Drama", "Science Fiction", "Mystery"]);
        assert!(n.plot.unwrap().starts_with("A linguist"));
    }

    #[test]
    fn ignores_url_only_nfo() {
        assert!(parse_nfo("https://www.imdb.com/title/tt2543164/").is_none());
    }
}
