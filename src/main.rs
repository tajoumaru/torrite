use anyhow::{Context, Result};
use clap::Parser;
use console::{style, Emoji};
use indicatif::HumanBytes;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use torrite::cli::{Cli, Commands, CreateArgs};
use torrite::models::TorrentSummary;
use torrite::TorrentBuilder;

mod verify;
mod edit;
mod inspect;

use verify::verify_torrent;
use edit::edit_torrent;
use inspect::inspect_torrent;

static SUCCESS: Emoji<'_, '_> = Emoji("✅ ", "OK");
static MAGNET: Emoji<'_, '_> = Emoji("🧲 ", "MAG");

fn main() -> Result<()> {
    // Check if the first argument is a known subcommand or help/version flag
    let args: Vec<String> = std::env::args().collect();
    let mut modified_args = args.clone();

    if args.len() > 1 {
        let first_arg = &args[1];
        if first_arg != "verify"
            && first_arg != "edit"
            && first_arg != "inspect"
            && first_arg != "create"
            && first_arg != "help"
            && first_arg != "--help"
            && first_arg != "-h"
            && first_arg != "--version"
            && first_arg != "-V"
        {
             // If it's not a known subcommand or flag, assume "create"
             modified_args.insert(1, "create".to_string());
        }
    }

    let cli = Cli::parse_from(modified_args);

    match cli.command {
        Commands::Create(args) => cmd_create(args),
        Commands::Verify(args) => verify_torrent(args),
        Commands::Edit(args) => edit_torrent(args),
        Commands::Inspect(args) => inspect_torrent(args),
    }
}

fn cmd_create(args: CreateArgs) -> Result<()> {
    let verbose = args.verbose;
    let force = args.force;
    let threads = args.threads;
    let show_info_hash = args.info_hash;
    let use_json = args.json;
    let source = args.source.clone();

    // Determine output file path
    let output_path = if let Some(path) = args.output.clone() {
        path
    } else {
        let name = args.name.clone().unwrap_or_else(|| {
            source
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("output")
                .to_string()
        });
        PathBuf::from(format!("{}.torrent", name))
    };

    let is_stdout = output_path.to_str() == Some("-");

    // Convert args to options
    let options = args.clone().into_options();
    let mode = options.mode; // Capture mode before options is moved into TorrentBuilder
    let is_dry_run = options.dry_run;

    // Build the torrent
    let mut builder = TorrentBuilder::new(source, options)
        .with_output_file(output_path.clone())
        .with_verbose(verbose)
        .with_progress(!use_json);

    if let Some(t) = threads {
        builder = builder.with_threads(t);
    }

    if is_dry_run {
        builder.dry_run()?;
        return Ok(());
    }

    let torrent = builder.build()?;

    // Serialize to bencode
    let bencode_data =
        serde_bencode::to_bytes(&torrent).context("Failed to serialize torrent to bencode")?;

    // Write to file or stdout
    if is_stdout {
        let mut stdout = std::io::stdout();
        stdout.write_all(&bencode_data).context("Failed to write torrent to stdout")?;
    } else {
        if verbose && !use_json {
            eprintln!("Writing to: {}", output_path.display());
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
    }

    if use_json {
        let summary = TorrentSummary {
            name: torrent.info.name.clone(),
            file_path: if is_stdout {
                "-".to_string()
            } else {
                output_path.to_string_lossy().into_owned()
            },
            total_size: torrent.total_size(),
            piece_length: torrent.info.piece_length,
            mode,
            info_hash_v1: torrent.info_hash_v1().map(hex::encode),
            info_hash_v2: torrent.info_hash_v2().map(hex::encode),
            magnet_link: torrent.magnet_link(),
        };
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else if !is_stdout {
        if verbose {
            eprintln!(
                "{} {}",
                SUCCESS,
                style(format!("Success! Torrent file created: {}", output_path.display())).green()
            );
        } else {
            eprintln!(
                "{} Created: {}",
                SUCCESS,
                style(output_path.display()).cyan()
            );
        }

        eprintln!("{:<12} {}", style("Name:").bold(), torrent.info.name);
        eprintln!("{:<12} {}", style("Total Size:").bold(), HumanBytes(torrent.total_size()));
        eprintln!("{:<12} {}", style("Piece Size:").bold(), HumanBytes(torrent.info.piece_length));

        if show_info_hash {
            if let Some(h1) = torrent.info_hash_v1() {
                eprintln!("{:<12} {}", style("Info Hash v1:").bold(), hex::encode(h1));
            }
            if let Some(h2) = torrent.info_hash_v2() {
                eprintln!("{:<12} {}", style("Info Hash v2:").bold(), hex::encode(h2));
            }
        }

        eprintln!("\n{} {}", MAGNET, style("Magnet Link:").bold());
        eprintln!("{}", style(torrent.magnet_link()).underlined());
    }

    Ok(())
}

