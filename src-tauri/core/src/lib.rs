//! Nsembe Play media engine.
//!
//! This crate is deliberately independent of Tauri (or any UI toolkit). It owns the
//! library database, filesystem scanning and watching, metadata extraction, artwork
//! generation and playback control. A client embeds it through [`engine::MediaEngine`]
//! and receives notifications through an [`events::EventSink`].

pub mod database;
pub mod engine;
pub mod errors;
pub mod events;
pub mod filesystem;
pub mod library;
pub mod metadata;
pub mod models;
pub mod player;
pub mod scanner;
pub mod settings;
pub mod thumbnails;
pub mod tools;
pub mod util;

pub use engine::{EngineConfig, MediaEngine};
pub use errors::{CoreError, Result};
pub use events::{CoreEvent, EventSink};
