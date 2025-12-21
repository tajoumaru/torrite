use anyhow::{Context, Result, anyhow};
use console::{style, Emoji};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::BTreeMap;

use torrite::cli::VerifyArgs;
use torrite::models::{Torrent, Info, FileInfo, Node};
use torrite::hashing::{hash_v1_pieces, hash_v2_files};

static SUCCESS: Emoji<'_, '_> = Emoji("✅ ", "OK");
static ERROR: Emoji<'_, '_> = Emoji("❌ ", "ERR");
static WARN: Emoji<'_, '_> = Emoji("⚠️ ", "WARN");

pub fn verify_torrent(args: VerifyArgs) -> Result<()> {
    // 1. Read torrent file
    let content = fs::read(&args.torrent).context("Failed to read torrent file")?;
    let torrent: Torrent = serde_bencode::from_bytes(&content).context("Invalid torrent file")?;

    // 2. Determine content root
    // If path is provided, use it.
    // If not, use current directory + name (common behavior for creating/verifying)
    // However, for single file torrents, it's often the file itself in cwd.
    let content_root = if let Some(path) = args.path {
        path
    } else {
        std::env::current_dir()?.join(&torrent.info.name)
    };

    println!("Verifying torrent: {}", style(&torrent.info.name).bold());
    println!("Content path: {}", style(content_root.display()).cyan());

    // 3. Build File List
    let files = build_file_list(&torrent.info, &content_root)?;

    if files.is_empty() {
        return Err(anyhow!("No files found in torrent info"));
    }

    // 4. Check existence and size
    check_files_exist(&files)?;

    // 5. Verify
    let mut v1_ok = true;
    let mut v2_ok = true;

    // V1 Verification
    if torrent.info.pieces.is_some() {
        println!("\n{}", style("Verifying V1 data...").bold());
        v1_ok = verify_v1(&torrent.info, &files)?;
    }

    // V2 Verification
    if torrent.info.meta_version == Some(2) {
         println!("\n{}", style("Verifying V2 data...").bold());
         v2_ok = verify_v2(&torrent.info, &files)?;
    } else if torrent.info.pieces.is_none() {
        println!("{}", style("No hash data found in torrent (neither V1 pieces nor V2 tree).").red());
        return Err(anyhow!("Invalid torrent: no hash data"));
    }

    if v1_ok && v2_ok {
        println!("\n{} {}", SUCCESS, style("Verification Successful!").green().bold());
    } else {
        println!("\n{} {}", ERROR, style("Verification Failed!").red().bold());
        // We don't bail here to allow caller to handle it, or we can exit with error.
        // The cli usually expects Result::Ok if command finished (even if verification failed? No, typically non-zero exit).
        return Err(anyhow!("Verification failed"));
    }

    Ok(())
}

fn build_file_list(info: &Info, content_root: &Path) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();
    let mut offset = 0;

    if let Some(ref file_entries) = info.files {
        // Multi-file mode (V1 compat)
        // Note: In multi-file mode, content_root is the directory.
        // Files are content_root/path/to/file
        for f in file_entries {
            let mut full_path = content_root.to_path_buf();
            let mut rel_path = PathBuf::new();
            for part in &f.path {
                full_path.push(part);
                rel_path.push(part);
            }
            
            files.push(FileInfo {
                path: rel_path,
                full_path,
                len: f.length,
                start_offset: offset,
                is_padding: f.attr.as_deref() == Some("p"),
            });
            offset += f.length;
        }
    } else if let Some(length) = info.length {
        // Single-file mode
        // content_root is the file itself.
        files.push(FileInfo {
            path: PathBuf::from(&info.name), // Relative path for V2 tree logic (will be ignored or used as root?)
            full_path: content_root.to_path_buf(),
            len: length,
            start_offset: 0,
            is_padding: false,
        });
    } else if let Some(ref tree) = info.file_tree {
        // V2 Only mode (no info.files)
        // We need to flatten the tree.
        // content_root is the directory.
        flatten_tree(tree, &PathBuf::new(), content_root, &mut files, &mut offset);
    } else {
        return Err(anyhow!("Invalid torrent info: missing files, length, or file tree"));
    }

    Ok(files)
}

fn flatten_tree(
    tree: &BTreeMap<String, Node>,
    rel_path: &PathBuf,
    base_path: &Path,
    files: &mut Vec<FileInfo>,
    offset: &mut u64,
) {
    for (name, node) in tree {
        let mut new_rel = rel_path.clone();
        if !name.is_empty() {
            new_rel.push(name);
        }

        let mut new_full = base_path.to_path_buf();
        if !name.is_empty() {
            new_full.push(name);
        }

        match node {
            Node::File(f) => {
                files.push(FileInfo {
                    path: new_rel,
                    full_path: new_full,
                    len: f.metadata.length,
                    start_offset: *offset,
                    is_padding: false, // V2 doesn't use padding files usually
                });
                *offset += f.metadata.length;
            }
            Node::Directory(sub_tree) => {
                flatten_tree(sub_tree, &new_rel, &new_full, files, offset);
            }
        }
    }
}

fn check_files_exist(files: &[FileInfo]) -> Result<()> {
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} Checking files")?
            .progress_chars("#>- ")
    );

    for file in files {
        if file.is_padding {
            continue;
        }
        if !file.full_path.exists() {
            pb.finish_and_clear();
            return Err(anyhow!("Missing file: {}", file.full_path.display()));
        }
        let metadata = fs::metadata(&file.full_path)
            .with_context(|| format!("Failed to stat file: {}", file.full_path.display()))?;
        
        if metadata.len() != file.len {
             pb.finish_and_clear();
             return Err(anyhow!(
                 "Size mismatch for file: {}. Expected {}, found {}",
                 file.full_path.display(),
                 file.len,
                 metadata.len()
             ));
        }
        pb.inc(1);
    }
    pb.finish_and_clear();
    println!("{} All files found and sizes match.", SUCCESS);
    Ok(())
}

fn verify_v1(info: &Info, files: &[FileInfo]) -> Result<bool> {
    let piece_length = info.piece_length;
    let expected_pieces = info.pieces.as_ref().unwrap(); // Safe because checked caller
    
    // Hash
    let pb = ProgressBar::new(expected_pieces.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} Verifying V1")? 
            .progress_chars("#>- ")
    );

    // Reuse existing hasher. It returns all hashes.
    // Note: this reads the whole file.
    // We pass pb to it for progress.
    let computed_hashes = hash_v1_pieces(files, piece_length, false, Some(pb))?;

    if computed_hashes.len() != expected_pieces.len() {
        println!("{} Hash length mismatch! Expected {}, got {}", ERROR, expected_pieces.len(), computed_hashes.len());
        return Ok(false);
    }

    let mut bad_pieces = 0;
    let num_pieces = expected_pieces.len() / 20;

    for i in 0..num_pieces {
        let start = i * 20;
        let end = start + 20;
        if computed_hashes[start..end] != expected_pieces[start..end] {
            bad_pieces += 1;
        }
    }

    if bad_pieces > 0 {
        println!("{} {} pieces corrupt out of {}", WARN, bad_pieces, num_pieces);
        return Ok(false);
    }

    println!("{} V1 verification passed.", SUCCESS);
    Ok(true)
}

fn verify_v2(info: &Info, files: &[FileInfo]) -> Result<bool> {
    let piece_length = info.piece_length;
    let expected_tree = info.file_tree.as_ref().context("Missing file tree for V2 torrent")?;

    // Hash
    // Actually we can sum files len.
    let total_size: u64 = files.iter().map(|f| f.len).sum();
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} Verifying V2")? 
            .progress_chars("#>- ")
    );

    let is_single_file = info.length.is_some() || (expected_tree.len() == 1 && expected_tree.contains_key(""));

    let result = hash_v2_files(files, piece_length, false, is_single_file, Some(pb))?;

    // Compare trees
    // We can't simply compare BTreeMaps because result.file_tree is constructed from files.
    // info.file_tree might contain directory structure.
    // hash_v2_files constructs the tree with the same structure if we used the same paths.
    // Since we built `files` from `info` (or compatible), the structure should match.
    
    // Using PartialEq we added to Node
    if &result.file_tree == expected_tree {
        println!("{} V2 verification passed.", SUCCESS);
        Ok(true)
    } else {
        println!("{} V2 Merkle tree mismatch.", ERROR);
        // We could traverse and find which file is bad, but for now just report failure.
        // To be more helpful:
        find_v2_mismatches(expected_tree, &result.file_tree, "");
        Ok(false)
    }
}

fn find_v2_mismatches(expected: &BTreeMap<String, Node>, actual: &BTreeMap<String, Node>, prefix: &str) {
    for (name, expected_node) in expected {
        let full_name: String = if prefix.is_empty() { name.clone() } else { format!("{}/{}", prefix, name) };
        if let Some(actual_node) = actual.get(name) {
            match (expected_node, actual_node) {
                (Node::File(ef), Node::File(af)) => {
                    if ef != af {
                         println!("  {} File corrupt: {}", ERROR, full_name);
                    }
                }
                (Node::Directory(ed), Node::Directory(ad)) => {
                    find_v2_mismatches(ed, ad, &full_name);
                }
                _ => {
                    println!("  {} Type mismatch for {}", ERROR, full_name);
                }
            }
        } else {
            println!("  {} Missing in result: {}", ERROR, full_name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use torrite::models::{FileMetadata, FileNode};
    use serde_bytes::ByteBuf;

    #[test]
    fn test_flatten_tree() {
        let mut tree = BTreeMap::new();
        // File 1: "a.txt"
        tree.insert("a.txt".to_string(), Node::File(FileNode {
            metadata: FileMetadata {
                length: 100,
                pieces_root: ByteBuf::new(),
            }
        }));
        
        // Directory: "b"
        let mut sub_tree = BTreeMap::new();
        // File 2: "b/c.txt"
        sub_tree.insert("c.txt".to_string(), Node::File(FileNode {
            metadata: FileMetadata {
                length: 200,
                pieces_root: ByteBuf::new(),
            }
        }));
        tree.insert("b".to_string(), Node::Directory(sub_tree));

        let mut files = Vec::new();
        let mut offset = 0;
        let base_path = Path::new("/base");

        flatten_tree(&tree, &PathBuf::new(), base_path, &mut files, &mut offset);

        assert_eq!(files.len(), 2);

        // Files are iterated in BTreeMap order (key order). "a.txt" comes before "b".
        let f1 = &files[0];
        assert_eq!(f1.path.to_str().unwrap(), "a.txt");
        assert_eq!(f1.full_path, base_path.join("a.txt"));
        assert_eq!(f1.len, 100);
        assert_eq!(f1.start_offset, 0);

        let f2 = &files[1];
        assert_eq!(f2.path.to_str().unwrap(), "b/c.txt");
        assert_eq!(f2.full_path, base_path.join("b/c.txt"));
        assert_eq!(f2.len, 200);
        assert_eq!(f2.start_offset, 100);
        
        assert_eq!(offset, 300);
    }
}
