use std::fs::File;
use torrite::{TorrentBuilder, TorrentOptions, Mode};

// Helper to create a dummy file of specific size
fn create_dummy_file(dir: &std::path::Path, name: &str, size: u64) -> std::path::PathBuf {
    let file_path = dir.join(name);
    let f = File::create(&file_path).unwrap();
    f.set_len(size).unwrap();
    file_path
}

#[test]
fn test_builder_applies_ptp_defaults() {
    let tmp_dir = tempfile::tempdir().unwrap();
    // 50 MB file -> Should trigger PTP 2^15 (32 KiB) or specific curve point?
    // PTP Config:
    // <= 58 MiB -> 2^16 (64 KiB)
    let file_path = create_dummy_file(tmp_dir.path(), "movie.mkv", 50 * 1024 * 1024);

    let mut options = TorrentOptions::default();
    options.mode = Mode::V1;
    options.announce = vec!["https://passthepopcorn.me/announce".to_string()];

    // We don't specify piece_length or source, expecting defaults

    let builder = TorrentBuilder::new(file_path, options);
    let torrent = builder.build().expect("Failed to build torrent");

    // Check Source
    assert_eq!(torrent.info.source, Some("PTP".to_string()));

    // Check Piece Length
    // 50 MiB is <= 58 MiB, so expected 2^16 = 65536
    assert_eq!(torrent.info.piece_length, 65536);
}

#[test]
fn test_builder_applies_anthelion_defaults() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let file_path = create_dummy_file(tmp_dir.path(), "movie.mkv", 10 * 1024 * 1024);

    let mut options = TorrentOptions::default();
    options.mode = Mode::V1;
    options.announce = vec!["https://anthelion.me/announce".to_string()];

    let builder = TorrentBuilder::new(file_path, options);
    let torrent = builder.build().expect("Failed to build torrent");

    assert_eq!(torrent.info.source, Some("ANT".to_string()));
}

#[test]
fn test_builder_caps_piece_size_for_ggn() {
    let tmp_dir = tempfile::tempdir().unwrap();
    // 100 GB file, would normally result in large pieces (e.g. 8MB or 16MB)
    // GGn max piece length is 2^26 (64 MiB), wait, checking src/trackers.rs...
    // GGn: max_piece_length: Some(26).
    // Let's try to force a situation where a default calculation might go high, 
    // or manually request something too high.
    
    let file_path = create_dummy_file(tmp_dir.path(), "game.iso", 1024 * 1024 * 1024); // 1 GB

    let mut options = TorrentOptions::default();
    options.mode = Mode::V1;
    options.announce = vec!["https://gazellegames.net/announce".to_string()];
    options.piece_length = Some(28); // Try to request 2^28 (256 MB)

    let builder = TorrentBuilder::new(file_path, options);
    let torrent = builder.build().expect("Failed to build torrent");

    // Should be capped at 26 (64 MB)
    // 2^26 = 67108864
    assert_eq!(torrent.info.piece_length, 67108864);
}

#[test]
fn test_builder_overrides_defaults_if_specified() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let file_path = create_dummy_file(tmp_dir.path(), "movie.mkv", 50 * 1024 * 1024);

    let mut options = TorrentOptions::default();
    options.mode = Mode::V1;
    options.announce = vec!["https://passthepopcorn.me/announce".to_string()];
    options.source_string = Some("MY_CUSTOM_SOURCE".to_string());
    options.piece_length = Some(18); // Force 2^18 = 256 KiB (instead of default 64 KiB)

    let builder = TorrentBuilder::new(file_path, options);
    let torrent = builder.build().expect("Failed to build torrent");

    assert_eq!(torrent.info.source, Some("MY_CUSTOM_SOURCE".to_string()));
    assert_eq!(torrent.info.piece_length, 262144);
}
