use std::fs::File;
use std::io::Write;
use torrite::{TorrentBuilder, TorrentOptions};

#[test]
fn test_torrent_metadata_options() {
    let tmp_dir = std::env::temp_dir().join("torrite_metadata");
    if tmp_dir.exists() { std::fs::remove_dir_all(&tmp_dir).unwrap(); }
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let file_path = tmp_dir.join("metadata.txt");
    File::create(&file_path).unwrap().write_all(b"Metadata").unwrap();

    let mut options = TorrentOptions::default();
    options.announce = vec!["http://tracker1.com".into(), "http://tracker2.com".into()];
    options.web_seed = vec!["http://webseed.com".into()];
    options.comment = Some("Test Comment".into());
    options.private = true;
    options.source_string = Some("SOURCE".into());
    options.name = Some("custom_name".into());
    options.creation_date = Some(1234567890);

    let builder = TorrentBuilder::new(file_path.clone(), options);
    let torrent = builder.build().unwrap();

    // Check Announce (first tier)
    assert_eq!(torrent.announce, Some("http://tracker1.com".to_string()));
    // Check Announce List (all tiers)
    let list = torrent.announce_list.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0][0], "http://tracker1.com");
    assert_eq!(list[1][0], "http://tracker2.com");

    // Check other metadata
    assert_eq!(torrent.url_list, Some(vec!["http://webseed.com".to_string()]));
    assert_eq!(torrent.comment, Some("Test Comment".to_string()));
    assert_eq!(torrent.info.private, Some(1));
    assert_eq!(torrent.info.source, Some("SOURCE".to_string()));
    assert_eq!(torrent.info.name, "custom_name");
    assert_eq!(torrent.creation_date, Some(1234567890));

    std::fs::remove_dir_all(&tmp_dir).unwrap();
}
