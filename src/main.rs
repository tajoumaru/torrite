use anyhow::{Context, Result};
use clap::Parser;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use torrite::TorrentBuilder;
use torrite::cli::Args;

fn main() -> Result<()> {
    let args = Args::parse();

    let verbose = args.verbose;
    let force = args.force;
    let threads = args.threads;
    let source = args.source.clone();

    // Determine output file path
    let output_path = args.output.clone().unwrap_or_else(|| {
        let name = args.name.clone().unwrap_or_else(|| {
            source
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("output")
                .to_string()
        });
        PathBuf::from(format!("{}.torrent", name))
    });

    // Convert args to options
    let options = args.into_options();

    // Build the torrent
    let mut builder = TorrentBuilder::new(source, options)
        .with_output_file(output_path.clone())
        .with_verbose(verbose);

    if let Some(t) = threads {
        builder = builder.with_threads(t);
    }

    let torrent = builder.build()?;

    // Serialize to bencode
    let bencode_data =
        serde_bencode::to_bytes(&torrent).context("Failed to serialize torrent to bencode")?;

    // Write to file
    if verbose {
        println!("Writing to: {}", output_path.display());
    }

    let mut output_file = if force {
        File::create(&output_path).context("Failed to create output file")?
    } else {
        File::options()
            .write(true)
            .create_new(true)
            .open(&output_path)
            .with_context(|| {
                format!(
                    "Failed to create output file (use -f to overwrite): {}",
                    output_path.display()
                )
            })?
    };

    output_file
        .write_all(&bencode_data)
        .context("Failed to write torrent file")?;

    if verbose {
        println!("Success! Torrent file created: {}", output_path.display());
    } else {
        println!("Created: {}", output_path.display());
    }

    Ok(())
}
