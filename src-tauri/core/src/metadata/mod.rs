//! Metadata extraction. Embedded metadata is always preferred; filename parsing is
//! the fallback. Online providers can be added later through [`provider::MetadataProvider`].

pub mod audio;
pub mod nfo;
pub mod provider;
pub mod subtitles;
pub mod video;
