use serde::Serialize;
use std::collections::BTreeMap;

use super::file::FileEntry;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    V1,
    V2,
    Hybrid,
}

#[derive(Debug, Serialize)]
pub struct FileMetadata {
    pub length: u64,
    #[serde(rename = "pieces root")]
    pub pieces_root: serde_bytes::ByteBuf,
}

#[derive(Debug, Serialize)]
pub struct FileNode {
    #[serde(rename = "")]
    pub metadata: FileMetadata,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Node {
    File(FileNode),
    Directory(BTreeMap<String, Node>),
}

/// Info dictionary for the torrent
#[derive(Debug, Serialize)]
pub struct Info {
    #[serde(rename = "piece length")]
    pub piece_length: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pieces: Option<serde_bytes::ByteBuf>,

    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub private: Option<u8>,

    // Multi-file mode (v1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<FileEntry>>,

    // Single-file mode (v1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u64>,

    // Source string (for cross-seeding)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    // Cross-seed random identifier (added to info dict to make hash unique)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_cross_seed: Option<String>,

    // v2 fields
    #[serde(rename = "meta version", skip_serializing_if = "Option::is_none")]
    pub meta_version: Option<u8>,

    #[serde(rename = "file tree", skip_serializing_if = "Option::is_none")]
    pub file_tree: Option<BTreeMap<String, Node>>,
}

/// Torrent metainfo structure
#[derive(Debug, Serialize)]
pub struct Torrent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub announce: Option<String>,

    #[serde(rename = "announce-list", skip_serializing_if = "Option::is_none")]
    pub announce_list: Option<Vec<Vec<String>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    #[serde(rename = "created by")]
    pub created_by: String,

    #[serde(rename = "creation date", skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<i64>,

    pub info: Info,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "url-list")]
    pub url_list: Option<Vec<String>>,

    #[serde(rename = "piece layers", skip_serializing_if = "Option::is_none")]
    pub piece_layers: Option<BTreeMap<serde_bytes::ByteBuf, serde_bytes::ByteBuf>>,
}

/// Configuration options for building a torrent
#[derive(Debug, Clone)]
pub struct TorrentOptions {
    pub mode: Mode,
    pub piece_length: Option<u32>,
    pub private: bool,
    pub comment: Option<String>,
    pub announce: Vec<String>,
    pub web_seed: Vec<String>,
    pub source_string: Option<String>,
    pub cross_seed: bool,
    pub no_date: bool,
    pub name: Option<String>,
    pub exclude: Vec<String>,
}

impl Default for TorrentOptions {
    fn default() -> Self {
        Self {
            mode: Mode::V1,
            piece_length: None,
            private: false,
            comment: None,
            announce: Vec::new(),
            web_seed: Vec::new(),
            source_string: None,
            cross_seed: false,
            no_date: false,
            name: None,
            exclude: Vec::new(),
        }
    }
}
