/// Block size for V2 hashing (16 KiB)
pub const BLOCK_SIZE: usize = 16384;

/// Megabyte constant for piece length calculations
pub const MB: u64 = 1_048_576;

/// Piece length thresholds for automatic calculation
/// Maps total size to piece length power (2^N)
pub const PIECE_LENGTH_THRESHOLDS: [(u64, u32); 9] = [
    (50 * MB, 15),    // <=50MB   -> 2^15 (32 KB)
    (100 * MB, 16),   // <=100MB  -> 2^16 (64 KB)
    (200 * MB, 17),   // <=200MB  -> 2^17 (128 KB)
    (400 * MB, 18),   // <=400MB  -> 2^18 (256 KB)
    (800 * MB, 19),   // <=800MB  -> 2^19 (512 KB)
    (1600 * MB, 20),  // <=1.6GB  -> 2^20 (1 MB)
    (3200 * MB, 21),  // <=3.2GB  -> 2^21 (2 MB)
    (6400 * MB, 22),  // <=6.4GB  -> 2^22 (4 MB)
    (12800 * MB, 23), // <=12.8GB -> 2^23 (8 MB)
];
