use clap::Parser;
use std::path::PathBuf;

use crate::models::{Mode, TorrentOptions};

#[derive(Parser, Debug)]
#[command(
    name = "mktorrent",
    version = "2.0.0",
    about = "A CLI utility to create BitTorrent metainfo files",
    author = "torrite contributors"
)]
pub struct Args {
    /// The file or directory to create a torrent from
    #[arg(value_name = "TARGET")]
    pub source: PathBuf,

    /// Announce URL(s) - can be specified multiple times for backup trackers
    #[arg(short = 'a', long = "announce", value_name = "URL")]
    pub announce: Vec<String>,

    /// Add a comment to the metainfo
    #[arg(short = 'c', long = "comment", value_name = "COMMENT")]
    pub comment: Option<String>,

    /// Don't write the creation date
    #[arg(short = 'd', long = "no-date")]
    pub no_date: bool,

    /// Exclude files matching pattern (glob) - can be comma-separated
    #[arg(short = 'e', long = "exclude", value_name = "PATTERN", value_delimiter = ',')]
    pub exclude: Vec<String>,

    /// Overwrite output file if it exists
    #[arg(short = 'f', long = "force")]
    pub force: bool,

    /// Set the piece length to 2^N bytes (e.g., 18 for 256KB)
    #[arg(short = 'l', long = "piece-length", value_name = "N")]
    pub piece_length: Option<u32>,

    /// Set the name of the torrent (defaults to basename of target)
    #[arg(short = 'n', long = "name", value_name = "NAME")]
    pub name: Option<String>,

    /// Set the output file path (defaults to <name>.torrent)
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Set the private flag
    #[arg(short = 'p', long = "private")]
    pub private: bool,

    /// Add source string embedded in infohash
    #[arg(short = 's', long = "source", value_name = "SOURCE")]
    pub source_string: Option<String>,

    /// Number of threads for hashing (defaults to number of CPU cores)
    #[arg(short = 't', long = "threads", value_name = "N")]
    pub threads: Option<usize>,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Web seed URL(s) - can be specified multiple times
    #[arg(short = 'w', long = "web-seed", value_name = "URL", value_delimiter = ',')]
    pub web_seed: Vec<String>,

    /// Ensure info hash is unique for easier cross-seeding
    #[arg(short = 'x', long = "cross-seed")]
    pub cross_seed: bool,

    /// Create a v2-only torrent (no v1 compatibility)
    #[arg(long = "v2", conflicts_with = "hybrid")]
    pub v2: bool,

    /// Create a hybrid torrent (v1 + v2 compatibility)
    #[arg(long = "hybrid", conflicts_with = "v2")]
    pub hybrid: bool,
}

impl Args {
    /// Convert CLI arguments to TorrentOptions
    pub fn into_options(self) -> TorrentOptions {
        let mode = if self.hybrid {
            Mode::Hybrid
        } else if self.v2 {
            Mode::V2
        } else {
            Mode::V1
        };

        TorrentOptions {
            mode,
            piece_length: self.piece_length,
            private: self.private,
            comment: self.comment,
            announce: self.announce,
            web_seed: self.web_seed,
            source_string: self.source_string,
            cross_seed: self.cross_seed,
            no_date: self.no_date,
            name: self.name,
            exclude: self.exclude,
        }
    }
}
