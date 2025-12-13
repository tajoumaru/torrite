use std::collections::BTreeMap;
use std::path::Path;

use crate::models::{FileMetadata, FileNode, Node};

/// Insert a file into the V2 file tree structure
pub fn insert_into_tree(
    tree: &mut BTreeMap<String, Node>,
    path: &Path,
    length: u64,
    root: Vec<u8>,
) {
    let components: Vec<_> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    insert_recursive(tree, &components, length, root);
}

fn insert_recursive(
    tree: &mut BTreeMap<String, Node>,
    components: &[String],
    length: u64,
    root: Vec<u8>,
) {
    if components.is_empty() {
        return;
    }

    let name = &components[0];

    if components.len() == 1 {
        tree.insert(
            name.clone(),
            Node::File(FileNode {
                metadata: FileMetadata {
                    length,
                    pieces_root: serde_bytes::ByteBuf::from(root),
                },
            }),
        );
    } else {
        let entry = tree
            .entry(name.clone())
            .or_insert_with(|| Node::Directory(BTreeMap::new()));
        if let Node::Directory(map) = entry {
            insert_recursive(map, &components[1..], length, root);
        }
    }
}
