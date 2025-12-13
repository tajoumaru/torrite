use crate::config::PIECE_LENGTH_THRESHOLDS;

/// Calculate optimal piece length based on total size (C-compatible algorithm)
pub fn calculate_piece_length(total_size: u64) -> u32 {
    // Find the appropriate piece length based on total size
    for (max_size, power) in PIECE_LENGTH_THRESHOLDS.iter() {
        if total_size <= *max_size {
            return *power;
        }
    }

    // For very large torrents (>12.8GB), use 8 MB pieces (2^23)
    23
}

/// Calculate the number of pieces for a given total size and piece length
pub fn calculate_num_pieces(total_size: u64, piece_length: u64) -> u64 {
    (total_size + piece_length - 1) / piece_length
}
