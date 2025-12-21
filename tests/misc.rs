use std::fs::File;
use std::io::Write;
use torrite::{TorrentBuilder, TorrentOptions};

#[test]
fn test_exclude_patterns() {
    let tmp_dir = std::env::temp_dir().join("torrite_exclude");
    if tmp_dir.exists() { std::fs::remove_dir_all(&tmp_dir).unwrap(); }
    std::fs::create_dir_all(&tmp_dir).unwrap();
    let content_dir = tmp_dir.join("exclude_content");
    std::fs::create_dir(&content_dir).unwrap();

    File::create(content_dir.join("keep.txt")).unwrap().write_all(b"keep").unwrap();
    File::create(content_dir.join("ignore.tmp")).unwrap().write_all(b"ignore").unwrap();
    File::create(content_dir.join("nested_ignore.tmp")).unwrap().write_all(b"ignore").unwrap();

    let mut options = TorrentOptions::default();
    options.exclude = vec!["*.tmp".into()];

    let builder = TorrentBuilder::new(content_dir, options);
    let torrent = builder.build().unwrap();

    let files = torrent.info.files.unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, vec!["keep.txt"]);

    std::fs::remove_dir_all(&tmp_dir).unwrap();
}

#[test]
fn test_piece_length_customization() {
    let tmp_dir = std::env::temp_dir().join("torrite_piece_len");
    if tmp_dir.exists() { std::fs::remove_dir_all(&tmp_dir).unwrap(); }
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let file_path = tmp_dir.join("data.bin");
    // Create 1MB file
    let data = vec![0u8; 1024 * 1024]; 
    File::create(&file_path).unwrap().write_all(&data).unwrap();

    let mut options = TorrentOptions::default();
    // 2^18 = 256KB
    options.piece_length = Some(18); 

    let builder = TorrentBuilder::new(file_path.clone(), options);
    let torrent = builder.build().unwrap();

    assert_eq!(torrent.info.piece_length, 262144); // 2^18
    
    // 1MB / 256KB = 4 pieces. 
    // SHA1 hash is 20 bytes. 4 * 20 = 80 bytes.
    let pieces = torrent.info.pieces.unwrap();
    assert_eq!(pieces.len(), 80);

    std::fs::remove_dir_all(&tmp_dir).unwrap();
}
