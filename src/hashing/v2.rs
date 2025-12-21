use anyhow::Result;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use indicatif::ProgressBar;

use crate::config::BLOCK_SIZE;
use crate::models::{FileInfo, FileMetadata, FileNode, Node};
use crate::tree::insert_into_tree;

/// Result of V2 hashing operation
pub struct V2HashResult {
    pub file_tree: BTreeMap<String, Node>,
    pub piece_layers: BTreeMap<serde_bytes::ByteBuf, serde_bytes::ByteBuf>,
}

struct FileResult {
    path: PathBuf,
    len: u64,
    root: Vec<u8>,
    layer_bytes: Option<Vec<u8>>,
}

/// Work item representing a chunk of blocks to hash from a single file
/// Each chunk is processed sequentially with one file handle
struct ChunkWork {
    file_index: usize,
    file_path: PathBuf,
    start_offset: u64,
    chunk_size: u64,
    start_block_index: usize,
}

/// Result of hashing a chunk (multiple blocks)
struct ChunkResult {
    file_index: usize,
    start_block_index: usize,
    hashes: Vec<[u8; 32]>,
}

/// Chunk size in bytes (128 blocks = 2MB)
/// This balances parallelism granularity with file I/O overhead
const CHUNK_SIZE_BLOCKS: usize = 128;
const CHUNK_SIZE_BYTES: u64 = (CHUNK_SIZE_BLOCKS * BLOCK_SIZE) as u64;

/// Hash all files using V2 SHA256 algorithm with merkle trees (block-parallel)
pub fn hash_v2_files(
    files: &[FileInfo],
    piece_length: u64,
    verbose: bool,
    is_single_file: bool,
    pb: Option<ProgressBar>,
) -> Result<V2HashResult> {
    if verbose && pb.is_none() {
        println!("  Computing V2 (SHA256) hashes and Merkle trees...");
    }

    let layer_index = if piece_length > BLOCK_SIZE as u64 {
        piece_length.trailing_zeros() as usize - BLOCK_SIZE.trailing_zeros() as usize
    } else {
        0
    };

    // Step 1: Build global work list of chunks across all files
    let mut work_list: Vec<ChunkWork> = Vec::new();

    for (file_index, file) in files.iter().enumerate() {
        if file.is_padding {
            continue;
        }

        if file.len == 0 {
            // Empty files will be handled separately, contribute no chunks
            continue;
        }

        // Split file into chunks of CHUNK_SIZE_BYTES
        let mut offset = 0u64;
        let mut block_index = 0usize;

        while offset < file.len {
            let remaining = file.len - offset;
            let chunk_size = std::cmp::min(CHUNK_SIZE_BYTES, remaining);

            work_list.push(ChunkWork {
                file_index,
                file_path: file.full_path.clone(),
                start_offset: offset,
                chunk_size,
                start_block_index: block_index,
            });

            let blocks_in_chunk = ((chunk_size + BLOCK_SIZE as u64 - 1) / BLOCK_SIZE as u64) as usize;
            block_index += blocks_in_chunk;
            offset += chunk_size;
        }
    }

    // Step 2: Process all chunks in parallel
    let chunk_results: Vec<ChunkResult> = work_list
        .par_iter()
        .map(|work| {
            // Open file and seek to chunk start
            let mut file = File::open(&work.file_path)
                .expect("Failed to open file for V2 hashing");
            file.seek(SeekFrom::Start(work.start_offset))
                .expect("Failed to seek in file");

            // Read and hash all blocks in this chunk sequentially
            let mut hashes = Vec::new();
            let mut buffer = vec![0u8; BLOCK_SIZE];
            let mut remaining = work.chunk_size;

            while remaining > 0 {
                let to_read = std::cmp::min(BLOCK_SIZE as u64, remaining) as usize;
                file.read_exact(&mut buffer[..to_read])
                    .expect("Failed to read file block");

                let mut hasher = Sha256::new();
                hasher.update(&buffer[..to_read]);
                hashes.push(hasher.finalize().into());

                if let Some(ref pb) = pb {
                    pb.inc(to_read as u64);
                }

                remaining -= to_read as u64;
            }

            ChunkResult {
                file_index: work.file_index,
                start_block_index: work.start_block_index,
                hashes,
            }
        })
        .collect();

    // Step 3: Reconstruct per-file results
    let mut file_hashes: BTreeMap<usize, Vec<(usize, [u8; 32])>> = BTreeMap::new();

    for result in chunk_results {
        let entry = file_hashes.entry(result.file_index).or_insert_with(Vec::new);
        for (i, hash) in result.hashes.into_iter().enumerate() {
            entry.push((result.start_block_index + i, hash));
        }
    }

    // Step 4: Build FileResult for each file
    let mut file_results: Vec<FileResult> = Vec::new();

    for (file_index, file) in files.iter().enumerate() {
        if file.is_padding {
            continue;
        }

        let hashes = if let Some(mut block_list) = file_hashes.remove(&file_index) {
            // Sort by block index to ensure correct order
            block_list.sort_by_key(|(block_idx, _)| *block_idx);
            block_list.into_iter().map(|(_, hash)| hash).collect()
        } else {
            // Empty file
            Vec::new()
        };

        let (root, layers) = compute_merkle_root(hashes);

        let mut layer_bytes = None;
        if file.len > piece_length {
            if let Some(layer) = layers.get(layer_index) {
                let mut lb = Vec::with_capacity(layer.len() * 32);
                for h in layer {
                    lb.extend_from_slice(h);
                }
                layer_bytes = Some(lb);
            }
        }

        file_results.push(FileResult {
            path: file.path.clone(),
            len: file.len,
            root: root.to_vec(),
            layer_bytes,
        });
    }

    // Assemble Tree
    let mut file_tree_nodes: BTreeMap<String, Node> = BTreeMap::new();
    let mut piece_layers: BTreeMap<serde_bytes::ByteBuf, serde_bytes::ByteBuf> = BTreeMap::new();

    for res in file_results {
        if let Some(lb) = res.layer_bytes {
            piece_layers.insert(
                serde_bytes::ByteBuf::from(res.root.clone()),
                serde_bytes::ByteBuf::from(lb),
            );
        }

        if is_single_file {
            file_tree_nodes.insert(
                "".to_string(),
                Node::File(FileNode {
                    metadata: FileMetadata {
                        length: res.len,
                        pieces_root: serde_bytes::ByteBuf::from(res.root),
                    },
                }),
            );
        } else {
            insert_into_tree(&mut file_tree_nodes, &res.path, res.len, res.root);
        }
    }

    Ok(V2HashResult {
        file_tree: file_tree_nodes,
        piece_layers,
    })
}

/// Compute Merkle Root and layers from block hashes
pub fn compute_merkle_root(hashes: Vec<[u8; 32]>) -> ([u8; 32], Vec<Vec<[u8; 32]>>) {
    if hashes.is_empty() {
        // Root of empty file is SHA256("")
        let empty_hash = Sha256::digest(&[]);
        return (empty_hash.into(), vec![vec![]]);
    }

    let mut layers = vec![hashes];
    while layers.last().unwrap().len() > 1 {
        let prev = layers.last().unwrap();
        let mut next = Vec::with_capacity((prev.len() + 1) / 2);
        for chunk in prev.chunks(2) {
            if chunk.len() == 2 {
                let mut hasher = Sha256::new();
                hasher.update(chunk[0]);
                hasher.update(chunk[1]);
                next.push(hasher.finalize().into());
            } else {
                next.push(chunk[0]);
            }
        }
        layers.push(next);
    }
    let root = layers.last().unwrap()[0];
    (root, layers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn test_compute_merkle_root() {
        // Test 1: Empty
        let (root, layers) = compute_merkle_root(vec![]);
        let expected_empty = Sha256::digest(&[]);
        let expected_root: [u8; 32] = expected_empty.into();
        assert_eq!(root, expected_root);
        assert_eq!(layers.len(), 1);

        // Test 2: Single block
        let h1 = [1u8; 32];
        let (root, layers) = compute_merkle_root(vec![h1]);
        assert_eq!(root, h1);
        assert_eq!(layers.len(), 1);

        // Test 3: Two blocks
        let h1 = [1u8; 32];
        let h2 = [2u8; 32];
        let (root, layers) = compute_merkle_root(vec![h1, h2]);
        
        let mut hasher = Sha256::new();
        hasher.update(h1);
        hasher.update(h2);
        let expected_root: [u8; 32] = hasher.finalize().into();
        
        assert_eq!(root, expected_root);
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0], vec![h1, h2]);
        assert_eq!(layers[1], vec![expected_root]);

        // Test 4: Three blocks (unbalanced)
        // Layer 0: [h1, h2, h3]
        // Layer 1: [H(h1+h2), h3]
        // Layer 2: [H(H(h1+h2)+h3)]
        let h3 = [3u8; 32];
        let (root, layers) = compute_merkle_root(vec![h1, h2, h3]);
        
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].len(), 3);
        assert_eq!(layers[1].len(), 2);
        assert_eq!(layers[2].len(), 1);
        
        // Check Layer 1
        let mut hasher = Sha256::new();
        hasher.update(h1);
        hasher.update(h2);
        let h12: [u8; 32] = hasher.finalize().into();
        assert_eq!(layers[1][0], h12);
        assert_eq!(layers[1][1], h3);

        // Check Root (Layer 2)
        let mut hasher = Sha256::new();
        hasher.update(h12);
        hasher.update(h3);
        let h123: [u8; 32] = hasher.finalize().into();
        assert_eq!(root, h123);
    }
}
