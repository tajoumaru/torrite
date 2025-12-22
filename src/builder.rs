use anyhow::Result;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
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
    show_progress: bool,
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
            show_progress: false,
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

    /// Enable progress bar
    pub fn with_progress(mut self, progress: bool) -> Self {
        self.show_progress = progress;
        self
    }

    /// Set the number of threads for hashing
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.num_threads = threads;
        self
    }

    /// Resolve tracker configuration based on announce URLs
    fn resolve_tracker_config(&self) -> Option<&'static crate::trackers::TrackerConfig> {
        if self.options.announce.is_empty() {
            return None;
        }
        // Check all announce URLs
        for tier_str in &self.options.announce {
            for url in tier_str.split(',') {
                if let Some(config) = crate::trackers::find_tracker_config(url.trim()) {
                    return Some(config);
                }
            }
        }
        None
    }

    /// Calculate piece length considering tracker configurations
    fn calculate_piece_length_with_config(
        &self,
        total_size: u64,
        config: Option<&crate::trackers::TrackerConfig>,
    ) -> (u64, u32) {
        // 1. User override
        if let Some(power) = self.options.piece_length {
            // Check max limit from config
            if let Some(cfg) = config {
                if let Some(max_exp) = cfg.max_piece_length {
                    if power > max_exp {
                        // Warn and cap
                        if self.verbose {
                            eprintln!(
                                "Warning: Requested piece length 2^{} exceeds tracker limit 2^{}. Capping.",
                                power, max_exp
                            );
                        }
                        return (1u64 << max_exp, max_exp);
                    }
                }
            }
            return (1u64 << power, power);
        }

        // 2. Config logic
        if let Some(cfg) = config {
            // Check ranges
            if !cfg.piece_size_ranges.is_empty() {
                for range in cfg.piece_size_ranges {
                    if total_size <= range.max_size {
                        let mut power = range.piece_exp;
                        // Enforce max limit
                        if let Some(max_exp) = cfg.max_piece_length {
                            if power > max_exp {
                                power = max_exp;
                            }
                        }
                        return (1u64 << power, power);
                    }
                }
                // No range match
                if !cfg.use_default_ranges {
                    // Use largest defined
                    let last = cfg.piece_size_ranges.last().unwrap();
                    let mut power = last.piece_exp;
                    if let Some(max_exp) = cfg.max_piece_length {
                        if power > max_exp {
                            power = max_exp;
                        }
                    }
                    return (1u64 << power, power);
                }
            } else if let Some(max_exp) = cfg.max_piece_length {
                // No ranges, but max limit. Use default calc but cap.
                let power = calculate_piece_length(total_size);
                let final_power = std::cmp::min(power, max_exp);
                return (1u64 << final_power, final_power);
            }
        }

        // 3. Default
        let power = calculate_piece_length(total_size);
        (1u64 << power, power)
    }

    /// Perform a dry run (scan files, calculate piece size, but don't hash)
    pub fn dry_run(&self) -> Result<()> {
        use console::{Emoji, style};
        use indicatif::HumanBytes;

        static DRY_RUN: Emoji<'_, '_> = Emoji("🏃 ", "DRY-RUN ");
        static CHECK: Emoji<'_, '_> = Emoji("✅ ", "OK ");
        static FILES: Emoji<'_, '_> = Emoji("📁 ", "f ");

        if self.verbose {
            eprintln!("torrite {} (Dry Run)", env!("CARGO_PKG_VERSION"));
            eprintln!();
            self.print_configuration();
        } else {
            eprintln!("{} {}", DRY_RUN, style("Dry run: scanning files...").bold());
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

        // Resolve tracker config
        let tracker_config = self.resolve_tracker_config();

        // Calculate or use provided piece length
        let (piece_length, power) =
            self.calculate_piece_length_with_config(total_size, tracker_config);

        let num_pieces = calculate_num_pieces(total_size, piece_length);

        eprintln!();
        eprintln!(
            "{} {}",
            CHECK,
            style("Dry Run Results:").bold().underlined()
        );
        eprintln!(
            "{:<15} {}",
            style("Total Size:").bold(),
            style(HumanBytes(total_size)).green()
        );
        eprintln!("{:<15} {}", style("File Count:").bold(), files.len());
        eprintln!(
            "{:<15} {} (2^{})",
            style("Piece Length:").bold(),
            style(HumanBytes(piece_length)).yellow(),
            power
        );
        eprintln!("{:<15} {}", style("Piece Count:").bold(), num_pieces);
        eprintln!("{:<15} {:?}", style("Mode:").bold(), self.options.mode);

        if self.verbose {
            eprintln!(
                "\n{} {}",
                FILES,
                style("Files that would be included:").bold()
            );
            for file in files.iter().take(20) {
                eprintln!(
                    "  - {:<40} {}",
                    file.path.display(),
                    style(HumanBytes(file.len)).dim()
                );
            }
            if files.len() > 20 {
                eprintln!("  ... and {} more", style(files.len() - 20).dim());
            }
        }

        Ok(())
    }

    /// Build the torrent metadata
    pub fn build(self) -> Result<Torrent> {
        if self.verbose {
            eprintln!("torrite {}", env!("CARGO_PKG_VERSION"));
            eprintln!();
            self.print_configuration();
        }

        // Scan files
        if self.verbose {
            eprintln!("Scanning files...");
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

        // Resolve tracker config
        let tracker_config = self.resolve_tracker_config();

        // Calculate or use provided piece length
        let (piece_length, power) =
            self.calculate_piece_length_with_config(total_size, tracker_config);

        if self.verbose {
            eprintln!("Using piece length: {} bytes (2^{})", piece_length, power);
        }

        let num_pieces = calculate_num_pieces(total_size, piece_length);
        if self.verbose {
            eprintln!("Total size: {} bytes", total_size);
            eprintln!("Number of pieces: {}", num_pieces);
            eprintln!();
            eprintln!("Using {} threads for hashing", self.num_threads);
            eprintln!("Mode: {:?}", self.options.mode);
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
            eprintln!("Building torrent file...");
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
        let total_size: u64 = files.iter().map(|f| f.len).sum();

        // Create thread pool once and use it for all hashing
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.num_threads)
            .build()
            .unwrap();

        pool.install(|| {
            // V1 HASHING
            let pieces_bytes = if self.options.mode != Mode::V2 {
                let pb = if self.show_progress {
                    let pb = ProgressBar::new(total_size);
                    pb.set_draw_target(ProgressDrawTarget::stderr_with_hz(10));
                    pb.set_style(ProgressStyle::with_template(
                        "{spinner:.green} [{elapsed_precise}] {bar:40.202/94} {bytes}/{total_bytes} ({eta}) {msg}"
                    )?
                    .progress_chars("█▓▒░"));
                    pb.set_message("Hashing V1...");
                    Some(pb)
                } else {
                    None
                };

                let res = hash_v1_pieces(files, piece_length, self.verbose, pb.clone())?;
                if let Some(p) = pb {
                    p.finish_with_message("V1 Hashing complete");
                }
                res
            } else {
                Vec::new()
            };

            // V2 HASHING
            let (file_tree, piece_layers, meta_version) =
                if self.options.mode == Mode::V2 || self.options.mode == Mode::Hybrid {
                    let pb = if self.show_progress {
                        let pb = ProgressBar::new(total_size);
                        pb.set_draw_target(ProgressDrawTarget::stderr_with_hz(10));
                        pb.set_style(ProgressStyle::with_template(
                            "{spinner:.green} [{elapsed_precise}] {bar:40.202/94} {bytes}/{total_bytes} ({eta}) {msg}"
                        )?
                        .progress_chars("█▓▒░"));
                        pb.set_message("Hashing V2...");
                        Some(pb)
                    } else {
                        None
                    };

                    let result = hash_v2_files(
                        files,
                        piece_length,
                        self.verbose,
                        is_single_file,
                        pb.clone(),
                    )?;
                    if let Some(p) = pb {
                        p.finish_with_message("V2 Hashing complete");
                    }
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

        // Resolve tracker config again
        let tracker_config = self.resolve_tracker_config();

        let source_string = if self.options.source_string.is_some() {
            self.options.source_string.clone()
        } else {
            tracker_config.and_then(|c| c.default_source.map(|s| s.to_string()))
        };

        let info = Info {
            piece_length,
            pieces: pieces_section,
            name: torrent_name.clone(),
            private: if self.options.private { Some(1) } else { None },
            files: files_section,
            length: length_section,
            source: source_string,
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
        } else {
            let mut list: Vec<Vec<String>> = Vec::new();
            for tier_str in &self.options.announce {
                let tier: Vec<String> = tier_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                if !tier.is_empty() {
                    list.push(tier);
                }
            }

            if list.is_empty() {
                (None, None)
            } else {
                let first_announce = list[0][0].clone();

                // If we have exactly one tier with one URL, we don't strictly need announce-list
                let single_tracker = list.len() == 1 && list[0].len() == 1;

                if single_tracker {
                    (Some(first_announce), None)
                } else {
                    (Some(first_announce), Some(list))
                }
            }
        };

        // Get creation date
        let creation_date = if self.options.no_date {
            None
        } else if let Some(timestamp) = self.options.creation_date {
            Some(timestamp)
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
            created_by: format!("torrite {}", env!("CARGO_PKG_VERSION")),
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
        eprintln!("Configuration:");
        eprintln!("  Source: {}", self.source.display());
        if let Some(ref output) = self.output_file {
            eprintln!("  Output: {}", output.display());
        }
        if let Some(ref name) = self.options.name {
            eprintln!("  Name: {}", name);
        }
        if !self.options.announce.is_empty() {
            eprintln!("  Announce URLs:");
            for (i, url) in self.options.announce.iter().enumerate() {
                eprintln!("    {}: {}", i + 1, url);
            }
        }
        if let Some(ref comment) = self.options.comment {
            eprintln!("  Comment: {}", comment);
        }
        eprintln!("  Private: {}", self.options.private);
        eprintln!("  No date: {}", self.options.no_date);
        if let Some(ref source) = self.options.source_string {
            eprintln!("  Source: {}", source);
        }
        if self.options.cross_seed {
            eprintln!("  Cross-seed: enabled");
        }
        eprintln!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TorrentOptions;
    use std::path::PathBuf;

    #[test]
    fn test_tracker_defaults_anthelion() {
        let mut options = TorrentOptions::default();
        options.announce = vec!["https://anthelion.me/announce".to_string()];

        let builder = TorrentBuilder::new(PathBuf::from("."), options);
        let config = builder.resolve_tracker_config().unwrap();
        assert_eq!(config.default_source, Some("ANT"));
        assert_eq!(config.max_torrent_size, Some(250 * 1024));
    }

    #[test]
    fn test_tracker_defaults_ptp() {
        let mut options = TorrentOptions::default();
        options.announce = vec!["https://passthepopcorn.me/announce".to_string()];

        let builder = TorrentBuilder::new(PathBuf::from("."), options);
        let config = builder.resolve_tracker_config().unwrap();
        assert_eq!(config.default_source, Some("PTP"));

        // Ranges:
        // {MaxSize: 58 << 20, PieceExp: 16},    // 64 KiB for <= 58 MiB
        // {MaxSize: 122 << 20, PieceExp: 17},   // 128 KiB for 58-122 MiB

        // 50 MiB -> 16
        let (len, pow) = builder.calculate_piece_length_with_config(50 * 1024 * 1024, Some(config));
        assert_eq!(pow, 16);
        assert_eq!(len, 1 << 16);

        // 100 MiB -> 17
        let (len, pow) =
            builder.calculate_piece_length_with_config(100 * 1024 * 1024, Some(config));
        assert_eq!(pow, 17);
        assert_eq!(len, 1 << 17);
    }

    #[test]
    fn test_tracker_defaults_ggn_max_limit() {
        let mut options = TorrentOptions::default();
        options.announce = vec!["https://gazellegames.net/announce".to_string()];

        // GGn has max piece length 26.
        let builder = TorrentBuilder::new(PathBuf::from("."), options.clone());
        let config = builder.resolve_tracker_config().unwrap();

        // If we request 28, it should cap at 26.
        let mut builder_override = TorrentBuilder::new(PathBuf::from("."), options.clone());
        builder_override.options.piece_length = Some(28);

        let (len, pow) = builder_override.calculate_piece_length_with_config(100, Some(config));
        assert_eq!(pow, 26);
        assert_eq!(len, 1 << 26);
    }
}
