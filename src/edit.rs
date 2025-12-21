use anyhow::{Context, Result};
use console::style;
use std::fs;

use torrite::cli::EditArgs;
use torrite::models::Torrent;

pub fn edit_torrent(args: EditArgs) -> Result<()> {
    let content = fs::read(&args.torrent).context("Failed to read torrent file")?;
    let mut torrent: Torrent = serde_bencode::from_bytes(&content).context("Invalid torrent file")?;

    if apply_changes(&mut torrent, &args) {
        let output_path = args.output.unwrap_or(args.torrent);
        println!("Saving to: {}", style(output_path.display()).cyan());

        let bencode_data = serde_bencode::to_bytes(&torrent).context("Failed to serialize torrent")?;
        fs::write(output_path, bencode_data).context("Failed to write torrent file")?;
    } else {
        println!("No changes made.");
    }

    Ok(())
}

fn apply_changes(torrent: &mut Torrent, args: &EditArgs) -> bool {
    let mut modified = false;

    // Announce
    if let Some(ref new_announce) = args.replace_announce {
        println!("Replaced announce with: {}", new_announce);
        torrent.announce = Some(new_announce.clone());
        torrent.announce_list = Some(vec![vec![new_announce.clone()]]);
        modified = true;
    } else if !args.announce.is_empty() {
        let mut list = torrent.announce_list.clone().unwrap_or_else(Vec::new);
        // Append as new tiers
        for url in &args.announce {
             println!("Added announce: {}", url);
             list.push(vec![url.clone()]);
        }
        // If main announce was empty, set it to the first one
        if torrent.announce.is_none() && !list.is_empty() {
            torrent.announce = Some(list[0][0].clone());
        }
        torrent.announce_list = Some(list);
        modified = true;
    }

    // Comment
    if let Some(ref comment) = args.comment {
        println!("Updated comment: {}", comment);
        torrent.comment = Some(comment.clone());
        modified = true;
    }

    // Private
    if args.private {
        if torrent.info.private != Some(1) {
            println!("Set private flag.");
            torrent.info.private = Some(1);
            modified = true;
        }
    } else if args.public {
         if torrent.info.private.is_some() {
             println!("Removed private flag.");
             torrent.info.private = None;
             modified = true;
         }
    }
    
    modified
}

#[cfg(test)]
mod tests {
    use super::*;
    use torrite::models::Info;
    use std::path::PathBuf;

    fn create_dummy_torrent() -> Torrent {
        Torrent {
            announce: None,
            announce_list: None,
            comment: None,
            created_by: "test".to_string(),
            creation_date: None,
            info: Info {
                piece_length: 1024,
                pieces: None,
                name: "test".to_string(),
                private: None,
                files: None,
                length: Some(100),
                source: None,
                x_cross_seed: None,
                meta_version: None,
                file_tree: None,
            },
            url_list: None,
            piece_layers: None,
        }
    }

    #[test]
    fn test_apply_changes_comment() {
        let mut torrent = create_dummy_torrent();
        let args = EditArgs {
            torrent: PathBuf::from("test.torrent"),
            announce: vec![],
            replace_announce: None,
            comment: Some("New Comment".to_string()),
            private: false,
            public: false,
            output: None,
        };
        
        assert!(apply_changes(&mut torrent, &args));
        assert_eq!(torrent.comment.unwrap(), "New Comment");
    }

    #[test]
    fn test_apply_changes_announce_replace() {
        let mut torrent = create_dummy_torrent();
        let args = EditArgs {
            torrent: PathBuf::from("test.torrent"),
            announce: vec![],
            replace_announce: Some("http://new.tracker".to_string()),
            comment: None,
            private: false,
            public: false,
            output: None,
        };

        assert!(apply_changes(&mut torrent, &args));
        assert_eq!(torrent.announce.unwrap(), "http://new.tracker");
        assert_eq!(torrent.announce_list.unwrap().len(), 1);
    }
    
    #[test]
    fn test_apply_changes_private() {
        let mut torrent = create_dummy_torrent();
        let args = EditArgs {
            torrent: PathBuf::from("test.torrent"),
            announce: vec![],
            replace_announce: None,
            comment: None,
            private: true,
            public: false,
            output: None,
        };
        
        assert!(apply_changes(&mut torrent, &args));
        assert_eq!(torrent.info.private, Some(1));
        
        // No change if already private
        assert!(!apply_changes(&mut torrent, &args));
    }
    
    #[test]
    fn test_apply_changes_public() {
        let mut torrent = create_dummy_torrent();
        torrent.info.private = Some(1);
        let args = EditArgs {
            torrent: PathBuf::from("test.torrent"),
            announce: vec![],
            replace_announce: None,
            comment: None,
            private: false,
            public: true,
            output: None,
        };
        
        assert!(apply_changes(&mut torrent, &args));
        assert_eq!(torrent.info.private, None);
    }
}
