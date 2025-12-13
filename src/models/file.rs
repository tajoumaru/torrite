use serde::Serialize;
use std::path::PathBuf;

/// Represents a file in the torrent
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Relative path for the torrent metadata
    pub path: PathBuf,
    /// Absolute path for reading the file
    pub full_path: PathBuf,
    /// File size in bytes
    pub len: u64,
    /// The byte offset where this file starts in the global stream
    pub start_offset: u64,
    /// Whether this is a padding file (virtual)
    pub is_padding: bool,
}

/// File entry in multi-file mode
#[derive(Debug, Serialize)]
pub struct FileEntry {
    pub length: u64,
    pub path: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attr: Option<String>,
}
