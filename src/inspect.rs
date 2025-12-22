use anyhow::{Context, Result};
use console::{style, Emoji};
use indicatif::HumanBytes;
use std::fs;

use torrite::cli::InspectArgs;
use torrite::models::Torrent;

static INFO: Emoji<'_, '_> = Emoji("ℹ️ ", "i ");
static FILES: Emoji<'_, '_> = Emoji("📁 ", "f ");
static TRACKERS: Emoji<'_, '_> = Emoji("📡 ", "t ");

pub fn inspect_torrent(args: InspectArgs) -> Result<()> {
    let path = args.torrent;
    let content = fs::read(&path).with_context(|| format!("Failed to read torrent file: {}", path.display()))?;

    let torrent: Torrent = serde_bencode::from_bytes(&content)
        .context("Failed to parse torrent file. Is it a valid bencoded file?")?;

    println!("{} {}", INFO, style("Torrent Metadata:").bold());
    println!("{:<15} {}", style("Name:").bold(), style(&torrent.info.name).cyan());
    
    if let Some(comment) = &torrent.comment {
         println!("{:<15} {}", style("Comment:").bold(), comment);
    }
    
    println!("{:<15} {}", style("Created By:").bold(), torrent.created_by);
    
    if let Some(date) = torrent.creation_date {
         let datetime = chrono::DateTime::from_timestamp(date, 0)
            .map(|dt| dt.to_string())
            .unwrap_or_else(|| date.to_string());
        println!("{:<15} {}", style("Date:").bold(), datetime);
    }

    println!("{:<15} {}", style("Total Size:").bold(), style(HumanBytes(torrent.total_size())).green());
    println!("{:<15} {}", style("Piece Size:").bold(), style(HumanBytes(torrent.info.piece_length)).yellow());
    
    if let Some(pieces) = &torrent.info.pieces {
        let num_pieces = pieces.len() / 20;
        println!("{:<15} {}", style("Piece Count:").bold(), num_pieces);
    }

    println!("{:<15} {}", style("Private:").bold(), if torrent.info.private.unwrap_or(0) == 1 { style("yes").red() } else { style("no").dim() });

    if let Some(v1_hash) = torrent.info_hash_v1() {
        println!("{:<15} {}", style("Info Hash v1:").bold(), hex::encode(v1_hash));
    }
    
    if let Some(v2_hash) = torrent.info_hash_v2() {
        println!("{:<15} {}", style("Info Hash v2:").bold(), hex::encode(v2_hash));
    }

    println!("\n{} {}", TRACKERS, style("Trackers:").bold());
    if let Some(announce) = &torrent.announce {
        println!("  - {}", style(announce).underlined());
    }
    
    if let Some(announce_list) = &torrent.announce_list {
        for tier in announce_list {
            for tracker in tier {
                if Some(tracker) != torrent.announce.as_ref() {
                    println!("  - {}", style(tracker).underlined());
                }
            }
        }
    }
    
    if let Some(web_seeds) = &torrent.url_list {
        println!("\n{}", style("Web Seeds:").bold());
        for url in web_seeds {
            println!("  - {}", style(url).underlined());
        }
    }

    println!("\n{} {}", FILES, style("Files:").bold());
    if let Some(files) = &torrent.info.files {
        for (i, file) in files.iter().enumerate() {
            if i >= 20 {
                println!("  ... and {} more files", style(files.len() - 20).dim());
                break;
            }
            let path = file.path.join("/");
            println!("  - {:<40} {}", path, style(HumanBytes(file.length)).dim());
        }
    } else if let Some(_tree) = &torrent.info.file_tree {
        println!("  {}", style("(V2 File Tree structure present)").italic().dim());
    } else {
        println!("  - {:<40} {}", torrent.info.name, style(HumanBytes(torrent.total_size())).dim());
    }

    Ok(())
}
