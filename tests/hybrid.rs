use std::fs::File;
use std::io::Write;
use torrite::{TorrentBuilder, TorrentOptions, Mode};

#[test]
fn test_generate_hybrid_single_file_torrent() {
    let tmp_dir = std::env::temp_dir().join("torrite_hybrid");
    if tmp_dir.exists() { std::fs::remove_dir_all(&tmp_dir).unwrap(); }
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let file_path = tmp_dir.join("hybrid_test.txt");
    let mut file = File::create(&file_path).unwrap();
    file.write_all(b"Hybrid Mode Content").unwrap();

    let mut options = TorrentOptions::default();
    options.mode = Mode::Hybrid;
    options.piece_length = Some(15);

    let builder = TorrentBuilder::new(file_path.clone(), options);
    let torrent = builder.build().unwrap();

    assert_eq!(torrent.info.name, "hybrid_test.txt");
    // Single file hybrid has V1 fields
    assert!(torrent.info.length.is_some());
    // And V2 fields
    assert_eq!(torrent.info.meta_version, Some(2));
    assert!(torrent.info.file_tree.is_some());

    // Should have both hashes
    assert!(torrent.info_hash_v1().is_some());
    assert!(torrent.info_hash_v2().is_some());

    std::fs::remove_dir_all(&tmp_dir).unwrap();
}
