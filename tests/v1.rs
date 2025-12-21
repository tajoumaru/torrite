use std::fs::File;
use std::io::Write;
use torrite::{TorrentBuilder, TorrentOptions, Mode};

#[test]
fn test_generate_single_file_torrent_v1() {
    // Setup
    let tmp_dir = std::env::temp_dir().join("torrite_v1_single");
    if tmp_dir.exists() {
        std::fs::remove_dir_all(&tmp_dir).unwrap();
    }
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let file_path = tmp_dir.join("test_file.txt");
    let mut file = File::create(&file_path).unwrap();
    file.write_all(b"Hello World! This is a test file for torrite.").unwrap();

    // Configure
    let mut options = TorrentOptions::default();
    options.mode = Mode::V1;
    options.piece_length = Some(15); // 2^15 = 32768 bytes, ensuring 1 piece

    // Build
    let builder = TorrentBuilder::new(file_path.clone(), options);
    let result = builder.build();

    // Assert
    assert!(result.is_ok());
    let torrent = result.unwrap();

    assert_eq!(torrent.info.name, "test_file.txt");
    assert_eq!(torrent.info.length, Some(45)); // Length of the string
    assert!(torrent.info.files.is_none()); // Single file mode

    // Check Info Hash presence
    assert!(torrent.info_hash_v1().is_some());
    assert!(torrent.info_hash_v2().is_none());

    // Cleanup
    std::fs::remove_dir_all(&tmp_dir).unwrap();
}

#[test]
fn test_generate_multi_file_torrent_v1() {
    // Setup
    let tmp_dir = std::env::temp_dir().join("torrite_v1_multi");
    if tmp_dir.exists() {
        std::fs::remove_dir_all(&tmp_dir).unwrap();
    }
    std::fs::create_dir_all(&tmp_dir).unwrap();
    
    let content_dir = tmp_dir.join("content");
    std::fs::create_dir(&content_dir).unwrap();

    let file1_path = content_dir.join("file1.txt");
    let mut file1 = File::create(&file1_path).unwrap();
    file1.write_all(b"File 1 content").unwrap();

    let file2_path = content_dir.join("file2.txt");
    let mut file2 = File::create(&file2_path).unwrap();
    file2.write_all(b"File 2 content").unwrap();

    // Configure
    let mut options = TorrentOptions::default();
    options.mode = Mode::V1;
    options.piece_length = Some(15);

    // Build
    let builder = TorrentBuilder::new(content_dir.clone(), options);
    let result = builder.build();

    // Assert
    assert!(result.is_ok());
    let torrent = result.unwrap();

    assert_eq!(torrent.info.name, "content");
    assert!(torrent.info.length.is_none()); // Multi file mode
    assert!(torrent.info.files.is_some());
    
    let files = torrent.info.files.as_ref().unwrap();
    assert_eq!(files.len(), 2);
    
    let has_file1 = files.iter().any(|f| f.path == vec!["file1.txt"]);
    let has_file2 = files.iter().any(|f| f.path == vec!["file2.txt"]);
    assert!(has_file1);
    assert!(has_file2);

    assert_eq!(torrent.total_size(), 14 + 14);

    // Cleanup
    std::fs::remove_dir_all(&tmp_dir).unwrap();
}
