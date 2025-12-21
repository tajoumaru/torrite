use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

use crate::models::{Mode, TorrentOptions};

#[derive(Parser, Debug, Clone)]
#[command(
    name = "torrite",
    version = "2.0.0",
    about = "A CLI utility to create BitTorrent metainfo files",
    author = "torrite contributors"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Create a new torrent (default)
    Create(CreateArgs),

    /// Verify local files against a torrent
    Verify(VerifyArgs),

    /// Edit an existing torrent's metadata
    Edit(EditArgs),
}

#[derive(Args, Debug, Clone)]
pub struct CreateArgs {
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

    /// Set the creation date (Unix timestamp)
    #[arg(long = "date", value_name = "TIMESTAMP")]
    pub date: Option<i64>,

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

    /// Display the info hash of the created torrent
    #[arg(long = "info-hash")]
    pub info_hash: bool,

    /// Output results in JSON format
    #[arg(long = "json")]
    pub json: bool,

    /// Create a v2-only torrent (no v1 compatibility)
    #[arg(long = "v2", conflicts_with = "hybrid")]
    pub v2: bool,

    /// Create a hybrid torrent (v1 + v2 compatibility)
    #[arg(long = "hybrid", conflicts_with = "v2")]
    pub hybrid: bool,
}

#[derive(Args, Debug, Clone)]
pub struct VerifyArgs {
    /// The torrent file to verify against
    #[arg(value_name = "TORRENT")]
    pub torrent: PathBuf,

    /// The path to the data directory or file (defaults to current directory)
    #[arg(long = "path", value_name = "PATH")]
    pub path: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub struct EditArgs {
    /// The torrent file to edit
    #[arg(value_name = "TORRENT")]
    pub torrent: PathBuf,

    /// Append announce URL(s)
    #[arg(short = 'a', long = "announce", value_name = "URL")]
    pub announce: Vec<String>,

    /// Replace all announce URLs with this one
    #[arg(long = "replace-announce", value_name = "URL", conflicts_with = "announce")]
    pub replace_announce: Option<String>,

    /// Set or update the comment
    #[arg(short = 'c', long = "comment", value_name = "COMMENT")]
    pub comment: Option<String>,

    /// Set the private flag
    #[arg(long = "private")]
    pub private: bool,

    /// Unset the private flag (make public)
    #[arg(long = "public", conflicts_with = "private")]
    pub public: bool,

    /// Set the output file path (defaults to overwriting input)
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub output: Option<PathBuf>,
}

impl CreateArgs {
    /// Convert CLI arguments to TorrentOptions
    pub fn into_options(self) -> TorrentOptions {
        let mode = if self.hybrid {
            Mode::Hybrid
        } else if self.v2 {
            Mode::V2
        } else {
            Mode::V1
        };

        let creation_date = self.date.or_else(|| {
            std::env::var("SOURCE_DATE_EPOCH")
                .ok()
                .and_then(|s| s.parse::<i64>().ok())
        });

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
            creation_date,
            name: self.name,
            exclude: self.exclude,
        }
    }
}
