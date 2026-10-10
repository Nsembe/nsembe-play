//! Extension point for metadata sources.
//!
//! The core application only ships offline providers (embedded tags, filenames and
//! local `.nfo` files). Online sources such as TMDB or MusicBrainz can implement
//! this trait later; the engine must keep working when they are unavailable.

use futures::future::BoxFuture;

use crate::errors::Result;

#[derive(Debug, Clone, Default)]
pub struct MovieQuery {
    pub title: String,
    pub year: Option<i64>,
    pub path: String,
}

#[derive(Debug, Clone, Default)]
pub struct MovieMetadata {
    pub title: Option<String>,
    pub year: Option<i64>,
    pub overview: Option<String>,
    pub genres: Vec<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AlbumQuery {
    pub title: String,
    pub artist: String,
}

#[derive(Debug, Clone, Default)]
pub struct AlbumMetadata {
    pub year: Option<i64>,
    pub genre: Option<String>,
    pub cover_url: Option<String>,
}

pub trait MetadataProvider: Send + Sync {
    /// Stable identifier, e.g. "nfo", "tmdb".
    fn id(&self) -> &'static str;
    /// Providers that need the network are skipped when offline or disabled.
    fn requires_network(&self) -> bool;
    fn movie(&self, query: &MovieQuery) -> BoxFuture<'_, Result<Option<MovieMetadata>>>;
    fn album(&self, _query: &AlbumQuery) -> BoxFuture<'_, Result<Option<AlbumMetadata>>> {
        Box::pin(async { Ok(None) })
    }
}

/// Offline provider reading Kodi/Jellyfin style `.nfo` files next to the media.
pub struct LocalNfoProvider;

impl MetadataProvider for LocalNfoProvider {
    fn id(&self) -> &'static str {
        "nfo"
    }

    fn requires_network(&self) -> bool {
        false
    }

    fn movie(&self, query: &MovieQuery) -> BoxFuture<'_, Result<Option<MovieMetadata>>> {
        let path = query.path.clone();
        Box::pin(async move {
            Ok(super::nfo::find_movie_nfo(std::path::Path::new(&path)).map(|n| MovieMetadata {
                title: n.title,
                year: n.year,
                overview: n.plot,
                genres: n.genres,
                poster_url: None,
                backdrop_url: None,
            }))
        })
    }
}
