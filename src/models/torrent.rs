use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use sha1::{Sha1, Digest};
use sha2::Sha256;

use super::file::FileEntry;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Mode {
    #[serde(rename = "v1")]
    V1,
    #[serde(rename = "v2")]
    V2,
    #[serde(rename = "hybrid")]
    Hybrid,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct FileMetadata {
    pub length: u64,
    #[serde(rename = "pieces root")]
    pub pieces_root: serde_bytes::ByteBuf,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct FileNode {
    #[serde(rename = "")]
    pub metadata: FileMetadata,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum Node {
    File(FileNode),
    Directory(BTreeMap<String, Node>),
}

/// Info dictionary for the torrent
#[derive(Debug, Serialize, Deserialize, Clone)]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
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

impl Torrent {
    pub fn info_hash_v1(&self) -> Option<[u8; 20]> {
        if self.info.meta_version == Some(2) && self.info.pieces.is_none() {
            return None;
        }
        let info_bytes = serde_bencode::to_bytes(&self.info).ok()?;
        let mut hasher = Sha1::new();
        hasher.update(&info_bytes);
        Some(hasher.finalize().into())
    }

    pub fn info_hash_v2(&self) -> Option<[u8; 32]> {
        if self.info.meta_version != Some(2) {
            return None;
        }
        let info_bytes = serde_bencode::to_bytes(&self.info).ok()?;
        let mut hasher = Sha256::new();
        hasher.update(&info_bytes);
        Some(hasher.finalize().into())
    }

    pub fn magnet_link(&self) -> String {
        let mut link = format!("magnet:?dn={}", urlencoding::encode(&self.info.name));

        if let Some(hash) = self.info_hash_v1() {
            link.push_str(&format!("&xt=urn:btih:{}", hex::encode(hash)));
        }

        if let Some(hash) = self.info_hash_v2() {
            link.push_str(&format!("&xt=urn:btmh:1220{}", hex::encode(hash)));
        }

        if let Some(ref announce) = self.announce {
            link.push_str(&format!("&tr={}", urlencoding::encode(announce)));
        }

        if let Some(ref list) = self.announce_list {
            for tier in list {
                for tr in tier {
                    link.push_str(&format!("&tr={}", urlencoding::encode(tr)));
                }
            }
        }

        link
    }

    pub fn total_size(&self) -> u64 {
        if let Some(len) = self.info.length {
            return len;
        }

        if let Some(ref files) = self.info.files {
            return files.iter().map(|f| f.length).sum();
        }

        if let Some(ref tree) = self.info.file_tree {
            return tree.values().map(|node| node.total_size()).sum();
        }

        0
    }
}

impl Node {
    pub fn total_size(&self) -> u64 {
        match self {
            Node::File(f) => f.metadata.length,
            Node::Directory(d) => d.values().map(|node| node.total_size()).sum(),
        }
    }
}

/// Summary of the created torrent for JSON output
#[derive(Debug, Serialize)]
pub struct TorrentSummary {
    pub name: String,
    pub file_path: String,
    pub total_size: u64,
    pub piece_length: u64,
    pub mode: Mode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_hash_v1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_hash_v2: Option<String>,
    pub magnet_link: String,
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
    pub creation_date: Option<i64>,
    pub name: Option<String>,
    pub exclude: Vec<String>,
    pub dry_run: bool,
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
            creation_date: None,
            name: None,
            exclude: Vec::new(),
            dry_run: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FileEntry;

    #[test]
    fn test_total_size_single_file() {
        let info = Info {
            piece_length: 1024,
            pieces: None,
            name: "test.iso".to_string(),
            private: None,
            files: None,
            length: Some(12345),
            source: None,
            x_cross_seed: None,
            meta_version: None,
            file_tree: None,
        };
        let torrent = Torrent {
            announce: None,
            announce_list: None,
            comment: None,
            created_by: "test".to_string(),
            creation_date: None,
            info,
            url_list: None,
            piece_layers: None,
        };
        assert_eq!(torrent.total_size(), 12345);
    }

    #[test]
    fn test_total_size_multi_file() {
         let info = Info {
            piece_length: 1024,
            pieces: None,
            name: "test_dir".to_string(),
            private: None,
            files: Some(vec![
                FileEntry { length: 100, path: vec!["a.txt".into()], attr: None },
                FileEntry { length: 200, path: vec!["b.txt".into()], attr: None },
            ]),
            length: None,
            source: None,
            x_cross_seed: None,
            meta_version: None,
            file_tree: None,
        };
        let torrent = Torrent {
            announce: None,
            announce_list: None,
            comment: None,
            created_by: "test".to_string(),
            creation_date: None,
            info,
            url_list: None,
            piece_layers: None,
        };
        assert_eq!(torrent.total_size(), 300);
    }

    #[test]
    fn test_magnet_link() {
        let info = Info {
            piece_length: 0,
            pieces: Some(serde_bytes::ByteBuf::from(vec![0; 20])), // Dummy pieces to allow hash
            name: "test_file".to_string(),
            private: None,
            files: None,
            length: Some(100),
            source: None,
            x_cross_seed: None,
            meta_version: None,
            file_tree: None,
        };
        let torrent = Torrent {
            announce: Some("http://tracker.com/announce".to_string()),
            announce_list: None,
            comment: None,
            created_by: "test".to_string(),
            creation_date: None,
            info,
            url_list: None,
            piece_layers: None,
        };
        
        let magnet = torrent.magnet_link();
        assert!(magnet.starts_with("magnet:?"));
        assert!(magnet.contains("dn=test_file"));
        assert!(magnet.contains("tr=http%3A%2F%2Ftracker.com%2Fannounce"));
        assert!(magnet.contains("xt=urn:btih:"));
    }
}
