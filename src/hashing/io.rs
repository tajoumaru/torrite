use anyhow::{Context, Result};
use std::cmp::{max, min};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use crate::models::FileInfo;

/// Read data for a specific piece, potentially spanning multiple files
pub fn read_piece_data(
    files: &[FileInfo],
    piece_index: usize,
    piece_length: u64,
    total_len: u64,
) -> Result<Vec<u8>> {
    let global_start = piece_index as u64 * piece_length;
    let expected_len = min(piece_length, total_len.saturating_sub(global_start));
    if expected_len == 0 {
        return Ok(Vec::new());
    }
    let global_end = global_start + expected_len;

    let mut buffer = vec![0u8; expected_len as usize];

    // Find the first file that overlaps with this piece
    // We want the first file where end_offset > global_start
    let start_file_idx = files.partition_point(|f| f.start_offset + f.len <= global_start);

    for file in &files[start_file_idx..] {
        if file.start_offset >= global_end {
            break;
        }

        let overlap_start = max(global_start, file.start_offset);
        let overlap_end = min(global_end, file.start_offset + file.len);

        if overlap_end > overlap_start {
            let buf_start = (overlap_start - global_start) as usize;
            let buf_end = (overlap_end - global_start) as usize;

            if file.is_padding {
                // Already zeroed
                continue;
            }

            let file_seek_pos = overlap_start - file.start_offset;

            let mut f = File::open(&file.full_path).with_context(|| {
                format!("Failed to open file: {}", file.full_path.display())
            })?;
            f.seek(SeekFrom::Start(file_seek_pos))?;
            f.read_exact(&mut buffer[buf_start..buf_end])?;
        }
    }
    Ok(buffer)
}
