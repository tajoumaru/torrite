use anyhow::{Context, Result};
use glob::Pattern;
use jwalk::WalkDir;
use std::path::{Path, PathBuf};

use crate::models::FileInfo;

/// Scans the source path and collects file information
pub fn scan_files(
    source: &Path,
    output_file: Option<&Path>,
    exclude_patterns: &[String],
    verbose: bool,
) -> Result<(Vec<FileInfo>, u64)> {
    let source = source
        .canonicalize()
        .context("Failed to resolve source path")?;

    let output_canonical = output_file.and_then(|p| p.canonicalize().ok());

    let mut files = Vec::new();
    let mut total_size = 0u64;

    // Compile glob patterns
    let mut patterns = Vec::new();
    for pattern_str in exclude_patterns {
        match Pattern::new(pattern_str) {
            Ok(p) => patterns.push(p),
            Err(e) => {
                if verbose {
                    eprintln!("Warning: Invalid glob pattern '{}': {}", pattern_str, e);
                }
            }
        }
    }

    if source.is_file() {
        // Single file mode
        let metadata = source.metadata().context("Failed to read file metadata")?;
        let len = metadata.len();

        files.push(FileInfo {
            path: source.file_name().context("Failed to get filename")?.into(),
            full_path: source.clone(),
            len,
            start_offset: 0,
            is_padding: false,
        });
        total_size = len;

        if verbose {
            eprintln!("Single file: {} ({} bytes)", source.display(), len);
        }
    } else {
        // Multi-file mode (directory)
        // Use jwalk for parallel traversal
        let base_path = &source;

        for entry in WalkDir::new(&source) {
            let entry = entry.context("Failed to read directory entry")?;
            let entry_path = entry.path();

            // jwalk returns directories too, skip them
            // entry.file_type() is typically available and cheap
            if entry.file_type().is_dir() {
                continue;
            }

            // Skip the output file if it's inside the source directory
            if let Some(ref output) = output_canonical {
                if entry_path == output.as_path() {
                    if verbose {
                        eprintln!("Skipping output file: {}", entry_path.display());
                    }
                    continue;
                }
            }

            // Get relative path from base
            // entry_path from jwalk is PathBuf (absolute if source is absolute?)
            // jwalk docs: "The path is relative to the current working directory, unless root path was absolute."
            // We canonicalized `source` above, so it is absolute. So `entry_path` is absolute.
            let relative_path = entry_path
                .strip_prefix(base_path)
                .context("Failed to create relative path")?;

            // Check exclude patterns
            let file_name = entry.file_name().to_string_lossy();
            let relative_path_str = relative_path.to_string_lossy();

            let should_exclude = patterns
                .iter()
                .any(|p| p.matches(&file_name) || p.matches(&relative_path_str));

            if should_exclude {
                if verbose {
                    eprintln!("Excluding: {}", entry_path.display());
                }
                continue;
            }

            let metadata = entry
                .metadata()
                .context("Failed to read file metadata")?;
            let len = metadata.len();

            files.push(FileInfo {
                path: relative_path.to_path_buf(),
                full_path: entry_path.to_path_buf(),
                len,
                start_offset: 0, // Placeholder
                is_padding: false,
            });

            total_size += len;

            if verbose {
                eprintln!("  {} ({} bytes)", relative_path.display(), len);
            }
        }

        if verbose {
            eprintln!(
                "Found {} files, total size: {} bytes",
                files.len(),
                total_size
            );
        }
    }

    // Sort files by path (critical for consistent info hash)
    files.sort_by(|a, b| a.path.cmp(&b.path));

    // Calculate start offsets strictly after sorting
    let mut current_offset = 0u64;
    for file in &mut files {
        file.start_offset = current_offset;
        current_offset += file.len;
    }

    // Sanity check
    if current_offset != total_size {
        if verbose {
            eprintln!(
                "Warning: Size mismatch after sorting? {} vs {}",
                current_offset, total_size
            );
        }
    }

    Ok((files, total_size))
}

/// Add padding files to align file boundaries with piece boundaries
pub fn add_padding_files(files: Vec<FileInfo>, piece_length: u64) -> Vec<FileInfo> {
    let mut new_files = Vec::with_capacity(files.len() * 2);
    let mut current_offset = 0;

    for (i, file) in files.iter().enumerate() {
        let mut f = file.clone();
        f.start_offset = current_offset;
        current_offset += f.len;
        new_files.push(f);

        // If it's the last file, no padding needed
        if i == files.len() - 1 {
            continue;
        }

        let remainder = file.len % piece_length;
        if remainder > 0 {
            let padding_len = piece_length - remainder;
            let padding_file = FileInfo {
                path: PathBuf::from(".pad").join(format!("{}", padding_len)),
                full_path: PathBuf::new(), // Dummy
                len: padding_len,
                start_offset: current_offset,
                is_padding: true,
            };
            current_offset += padding_len;
            new_files.push(padding_file);
        }
    }
    new_files
}

/// Generate random hex string for cross-seeding
pub fn generate_cross_seed_id() -> String {
    use rand::Rng;

    const RAND_LENGTH: usize = 16; // 16 bytes = 32 hex chars
    let mut rng = rand::rng();
    let mut random_bytes = [0u8; RAND_LENGTH];
    rng.fill(&mut random_bytes);

    let hex_string: String = random_bytes.iter().map(|b| format!("{:02X}", b)).collect();

    format!("mktorrent-{}", hex_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_padding_files() {
        let files = vec![
            FileInfo {
                path: PathBuf::from("a.txt"),
                full_path: PathBuf::from("/a.txt"),
                len: 100,
                start_offset: 0,
                is_padding: false,
            },
            FileInfo {
                path: PathBuf::from("b.txt"),
                full_path: PathBuf::from("/b.txt"),
                len: 200,
                start_offset: 0,
                is_padding: false,
            },
        ];
        let piece_length = 50;

        // 100 % 50 == 0 -> No padding
        // 200 % 50 == 0 -> No padding
        let padded = add_padding_files(files.clone(), piece_length);
        assert_eq!(padded.len(), 2);
        assert_eq!(padded[0].len, 100);
        assert_eq!(padded[1].len, 200);

        let piece_length = 60;
        // 100 % 60 = 40 -> Need 20 padding
        // 200 (last file) -> No padding
        let padded = add_padding_files(files.clone(), piece_length);
        assert_eq!(padded.len(), 3);
        
        assert_eq!(padded[0].path.to_str().unwrap(), "a.txt");
        assert_eq!(padded[0].len, 100);
        
        // Padding file
        assert!(padded[1].is_padding);
        assert_eq!(padded[1].len, 20);
        assert!(padded[1].path.starts_with(".pad"));
        
        assert_eq!(padded[2].path.to_str().unwrap(), "b.txt");
        assert_eq!(padded[2].len, 200);
        // Offset check
        assert_eq!(padded[1].start_offset, 100);
        assert_eq!(padded[2].start_offset, 120);
    }
}
