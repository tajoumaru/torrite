//! # mktorrent
//!
//! A library for creating BitTorrent metainfo files.
//!
//! This library provides functionality to create BitTorrent v1, v2, and hybrid torrents
//! with support for both single-file and multi-file torrents.
//!
//! ## Example
//!
//! ```no_run
//! use torrite::{TorrentBuilder, TorrentOptions};
//! use std::path::PathBuf;
//!
//! let options = TorrentOptions::default();
//! let builder = TorrentBuilder::new(PathBuf::from("my_file.txt"), options);
//! let torrent = builder.build().unwrap();
//! ```

pub mod builder;
pub mod cli;
pub mod config;
pub mod hashing;
pub mod models;
pub mod piece;
pub mod scanner;
pub mod trackers;
pub mod tree;

// Re-export main types for convenience
pub use builder::TorrentBuilder;
pub use models::{Mode, Torrent, TorrentOptions};
