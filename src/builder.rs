use anyhow::Result;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::hashing::{hash_v1_pieces, hash_v2_files};
use crate::models::{FileEntry, Info, Mode, Torrent, TorrentOptions};
use crate::piece::{calculate_num_pieces, calculate_piece_length};
use crate::scanner::{add_padding_files, generate_cross_seed_id, scan_files};

/// Builder for creating torrent files
pub struct TorrentBuilder {
    source: PathBuf,
    output_file: Option<PathBuf>,
    options: TorrentOptions,
    verbose: bool,
    num_threads: usize,
}

impl TorrentBuilder {
    /// Create a new TorrentBuilder
    pub fn new(source: PathBuf, options: TorrentOptions) -> Self {
        Self {
            source,
            output_file: None,
            options,
            verbose: false,
            num_threads: num_cpus::get(),
        }
    }

    /// Set the output file path for exclusion from scanning
    pub fn with_output_file(mut self, output: PathBuf) -> Self {
        self.output_file = Some(output);
        self
    }

    /// Enable verbose output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Set the number of threads for hashing
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.num_threads = threads;
        self
    }

    /// Build the torrent metadata
    pub fn build(self) -> Result<Torrent> {
        if self.verbose {
            println!("mktorrent-rs 2.0.0");
            println!();
            self.print_configuration();
        }

        // Scan files
        if self.verbose {
            println!("Scanning files...");
        }

        let (files, total_size) = scan_files(
            &self.source,
            self.output_file.as_deref(),
            &self.options.exclude,
            self.verbose,
        )?;

        if files.is_empty() {
            anyhow::bail!("No files found to create torrent from");
        }

        // Calculate or use provided piece length
        let piece_length = if let Some(power) = self.options.piece_length {
            // Validate user-provided piece length
            if power < 15 || power > 28 {
                anyhow::bail!("piece length must be between 15 and 28 (2^15 to 2^28 bytes)");
            }
            let len = 1u64 << power;
            if self.verbose {
                println!("Using piece length: {} bytes (2^{})", len, power);
            }
            len
        } else {
            let power = calculate_piece_length(total_size);
            let len = 1u64 << power;
            if self.verbose {
                println!("Calculated piece length: {} bytes (2^{})", len, power);
            }
            len
        };

        let num_pieces = calculate_num_pieces(total_size, piece_length);
        if self.verbose {
            println!("Total size: {} bytes", total_size);
            println!("Number of pieces: {}", num_pieces);
            println!();
            println!("Using {} threads for hashing", self.num_threads);
            println!("Mode: {:?}", self.options.mode);
        }

        let is_single_file = self.source.is_file();

        // Prepare files (inject padding if Hybrid)
        // V2-only does not use padding. V1 does not use padding (files are continuous).
        let files = if self.options.mode == Mode::Hybrid && !is_single_file {
            add_padding_files(files, piece_length)
        } else {
            files
        };

        // Hashing
        let (pieces_bytes, file_tree, piece_layers, meta_version) =
            self.hash_content(&files, piece_length, is_single_file)?;

        if self.verbose {
            println!("Building torrent file...");
        }

        // Build the torrent
        let torrent = self.build_torrent(
            &files,
            total_size,
            piece_length,
            is_single_file,
            pieces_bytes,
            file_tree,
            piece_layers,
            meta_version,
        )?;

        Ok(torrent)
    }

    fn hash_content(
        &self,
        files: &[crate::models::FileInfo],
        piece_length: u64,
        is_single_file: bool,
    ) -> Result<(
        Vec<u8>,
        Option<std::collections::BTreeMap<String, crate::models::Node>>,
        Option<std::collections::BTreeMap<serde_bytes::ByteBuf, serde_bytes::ByteBuf>>,
        Option<u8>,
    )> {
        if self.verbose {
            println!("Hashing content with {} threads...", self.num_threads);
        }

        // Create thread pool once and use it for all hashing
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.num_threads)
            .build()
            .unwrap();

        pool.install(|| {
            // V1 HASHING
            let pieces_bytes = if self.options.mode != Mode::V2 {
                hash_v1_pieces(files, piece_length, self.verbose)?
            } else {
                Vec::new()
            };

            // V2 HASHING
            let (file_tree, piece_layers, meta_version) =
                if self.options.mode == Mode::V2 || self.options.mode == Mode::Hybrid {
                    let result = hash_v2_files(
                        files,
                        piece_length,
                        self.verbose,
                        is_single_file,
                    )?;
                    (Some(result.file_tree), Some(result.piece_layers), Some(2))
                } else {
                    (None, None, None)
                };

            Ok((pieces_bytes, file_tree, piece_layers, meta_version))
        })
    }

    fn build_torrent(
        &self,
        files: &[crate::models::FileInfo],
        total_size: u64,
        piece_length: u64,
        is_single_file: bool,
        pieces_bytes: Vec<u8>,
        file_tree: Option<std::collections::BTreeMap<String, crate::models::Node>>,
        piece_layers: Option<
            std::collections::BTreeMap<serde_bytes::ByteBuf, serde_bytes::ByteBuf>,
        >,
        meta_version: Option<u8>,
    ) -> Result<Torrent> {
        // Determine torrent name
        let torrent_name = self.options.name.clone().unwrap_or_else(|| {
            self.source
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("output")
                .to_string()
        });

        // Determine files/length fields
        let (files_section, length_section) = if self.options.mode == Mode::V2 {
            // V2 (single or multi) does not use 'files' or 'length' in info dict (uses file tree)
            (None, None)
        } else if is_single_file {
            // V1/Hybrid Single File
            (None, Some(total_size))
        } else {
            // V1/Hybrid Multi File
            let file_entries: Vec<FileEntry> = files
                .iter()
                .map(|f| {
                    let path_components: Vec<String> = f
                        .path
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy().to_string())
                        .collect();

                    FileEntry {
                        length: f.len,
                        path: path_components,
                        attr: if f.is_padding {
                            Some("p".to_string())
                        } else {
                            None
                        },
                    }
                })
                .collect();
            (Some(file_entries), None)
        };

        // Determine pieces field
        let pieces_section = if self.options.mode == Mode::V2 {
            None
        } else {
            Some(serde_bytes::ByteBuf::from(pieces_bytes))
        };

        let info = Info {
            piece_length,
            pieces: pieces_section,
            name: torrent_name.clone(),
            private: if self.options.private { Some(1) } else { None },
            files: files_section,
            length: length_section,
            source: self.options.source_string.clone(),
            x_cross_seed: if self.options.cross_seed {
                Some(generate_cross_seed_id())
            } else {
                None
            },
            meta_version,
            file_tree,
        };

        // Build announce-list if multiple trackers are provided
        let (announce, announce_list) = if self.options.announce.is_empty() {
            (None, None)
        } else if self.options.announce.len() == 1 {
            (Some(self.options.announce[0].clone()), None)
        } else {
            let list: Vec<Vec<String>> = self
                .options
                .announce
                .iter()
                .map(|url| vec![url.clone()])
                .collect();
            (Some(self.options.announce[0].clone()), Some(list))
        };

        // Get creation date
        let creation_date = if self.options.no_date {
            None
        } else {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .map(|d| d.as_secs() as i64)
        };

        // Build the Torrent structure
        let torrent = Torrent {
            announce,
            announce_list,
            comment: self.options.comment.clone(),
            created_by: format!("mktorrent-rs {}", env!("CARGO_PKG_VERSION")),
            creation_date,
            info,
            url_list: if self.options.web_seed.is_empty() {
                None
            } else {
                Some(self.options.web_seed.clone())
            },
            piece_layers,
        };

        Ok(torrent)
    }

    fn print_configuration(&self) {
        println!("Configuration:");
        println!("  Source: {}", self.source.display());
        if let Some(ref output) = self.output_file {
            println!("  Output: {}", output.display());
        }
        if let Some(ref name) = self.options.name {
            println!("  Name: {}", name);
        }
        if !self.options.announce.is_empty() {
            println!("  Announce URLs:");
            for (i, url) in self.options.announce.iter().enumerate() {
                println!("    {}: {}", i + 1, url);
            }
        }
        if let Some(ref comment) = self.options.comment {
            println!("  Comment: {}", comment);
        }
        println!("  Private: {}", self.options.private);
        println!("  No date: {}", self.options.no_date);
        if let Some(ref source) = self.options.source_string {
            println!("  Source: {}", source);
        }
        if self.options.cross_seed {
            println!("  Cross-seed: enabled");
        }
        println!();
    }
}
