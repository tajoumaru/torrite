use anyhow::Result;
use rayon::prelude::*;
use sha1::{Digest, Sha1};

use crate::models::FileInfo;
use super::io::read_piece_data;

/// Hash all pieces using V1 SHA1 algorithm (piece-parallel)
pub fn hash_v1_pieces(
    files: &[FileInfo],
    piece_length: u64,
    verbose: bool,
) -> Result<Vec<u8>> {
    if verbose {
        println!("  Computing V1 (SHA1) hashes...");
    }

    let total_len: u64 = files.iter().map(|f| f.len).sum();
    let num_pieces = (total_len + piece_length - 1) / piece_length;

    let results: Vec<_> = (0..num_pieces)
        .into_par_iter()
        .map(|piece_idx| {
            let data = read_piece_data(files, piece_idx as usize, piece_length, total_len)
                .expect("Failed to read piece data");

            let mut hasher = Sha1::new();
            hasher.update(&data);
            let v1_hash = hasher.finalize();
            let mut v1_hash_arr = [0u8; 20];
            v1_hash_arr.copy_from_slice(&v1_hash);
            v1_hash_arr
        })
        .collect();

    let mut bytes = Vec::with_capacity((num_pieces as usize) * 20);
    for hash in results {
        bytes.extend_from_slice(&hash);
    }
    Ok(bytes)
}
