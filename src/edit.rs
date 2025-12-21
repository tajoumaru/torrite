use anyhow::{Context, Result};
use console::style;
use std::fs;

use torrite::cli::EditArgs;
use torrite::models::Torrent;

pub fn edit_torrent(args: EditArgs) -> Result<()> {
    let content = fs::read(&args.torrent).context("Failed to read torrent file")?;
    let mut torrent: Torrent = serde_bencode::from_bytes(&content).context("Invalid torrent file")?;

    let mut modified = false;

    // Announce
    if let Some(new_announce) = args.replace_announce {
        println!("Replaced announce with: {}", new_announce);
        torrent.announce = Some(new_announce.clone());
        torrent.announce_list = Some(vec![vec![new_announce]]);
        modified = true;
    } else if !args.announce.is_empty() {
        let mut list = torrent.announce_list.unwrap_or_else(Vec::new);
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
    if let Some(comment) = args.comment {
        println!("Updated comment: {}", comment);
        torrent.comment = Some(comment);
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

    if !modified {
        println!("No changes made.");
        return Ok(());
    }

    let output_path = args.output.unwrap_or(args.torrent);
    println!("Saving to: {}", style(output_path.display()).cyan());

    let bencode_data = serde_bencode::to_bytes(&torrent).context("Failed to serialize torrent")?;
    fs::write(output_path, bencode_data).context("Failed to write torrent file")?;

    Ok(())
}
