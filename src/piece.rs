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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MB;

    #[test]
    fn test_calculate_piece_length() {
        // Test boundaries
        assert_eq!(calculate_piece_length(0), 15);
        assert_eq!(calculate_piece_length(50 * MB), 15);
        assert_eq!(calculate_piece_length(50 * MB + 1), 16);
        
        assert_eq!(calculate_piece_length(100 * MB), 16);
        assert_eq!(calculate_piece_length(100 * MB + 1), 17);

        assert_eq!(calculate_piece_length(200 * MB), 17);
        assert_eq!(calculate_piece_length(200 * MB + 1), 18);

        assert_eq!(calculate_piece_length(12800 * MB), 23);
        assert_eq!(calculate_piece_length(12800 * MB + 1), 23);
        assert_eq!(calculate_piece_length(20000 * MB), 23);
    }

    #[test]
    fn test_calculate_num_pieces() {
        assert_eq!(calculate_num_pieces(0, 1024), 0);
        assert_eq!(calculate_num_pieces(100, 100), 1);
        assert_eq!(calculate_num_pieces(101, 100), 2);
        assert_eq!(calculate_num_pieces(1024, 1024), 1);
        assert_eq!(calculate_num_pieces(2048, 1024), 2);
        assert_eq!(calculate_num_pieces(2049, 1024), 3);
    }
}
