use std::fs::File;
use std::io::Write;
use torrite::{TorrentBuilder, TorrentOptions, Mode};

#[test]
fn test_generate_single_file_torrent_v2() {
    // Setup
    let tmp_dir = std::env::temp_dir().join("torrite_v2_single");
    if tmp_dir.exists() {
        std::fs::remove_dir_all(&tmp_dir).unwrap();
    }
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let file_path = tmp_dir.join("test_v2.txt");
    let mut file = File::create(&file_path).unwrap();
    file.write_all(b"V2 Content test").unwrap();

    // Configure
    let mut options = TorrentOptions::default();
    options.mode = Mode::V2;
    options.piece_length = Some(15); 

    // Build
    let builder = TorrentBuilder::new(file_path.clone(), options);
    let result = builder.build();

    // Assert
    assert!(result.is_ok());
    let torrent = result.unwrap();

    assert_eq!(torrent.info.name, "test_v2.txt");
    assert!(torrent.info.length.is_none()); // V2 doesn't use length in info dict like V1
    assert!(torrent.info.files.is_none()); 
    assert_eq!(torrent.info.meta_version, Some(2));
    assert!(torrent.info.file_tree.is_some());

    // Check Info Hash V2 presence
    assert!(torrent.info_hash_v2().is_some());
    // V2 only mode (not hybrid) shouldn't have v1 info hash
    assert!(torrent.info_hash_v1().is_none());

    // Cleanup
    std::fs::remove_dir_all(&tmp_dir).unwrap();
}
