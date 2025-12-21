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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_insert_into_tree_single_file() {
        let mut tree = BTreeMap::new();
        let path = PathBuf::from("test_file.txt");
        let root = vec![1, 2, 3];
        insert_into_tree(&mut tree, &path, 100, root.clone());

        assert_eq!(tree.len(), 1);
        if let Some(Node::File(f)) = tree.get("test_file.txt") {
            assert_eq!(f.metadata.length, 100);
            assert_eq!(f.metadata.pieces_root.as_ref(), &root);
        } else {
            panic!("Expected file node");
        }
    }

    #[test]
    fn test_insert_into_tree_nested_file() {
        let mut tree = BTreeMap::new();
        let path = PathBuf::from("dir1/dir2/test_file.txt");
        let root = vec![4, 5, 6];
        insert_into_tree(&mut tree, &path, 200, root.clone());

        assert_eq!(tree.len(), 1);
        
        // Check dir1
        let dir1 = match tree.get("dir1") {
            Some(Node::Directory(map)) => map,
            _ => panic!("Expected directory dir1"),
        };
        assert_eq!(dir1.len(), 1);

        // Check dir2
        let dir2 = match dir1.get("dir2") {
            Some(Node::Directory(map)) => map,
            _ => panic!("Expected directory dir2"),
        };
        assert_eq!(dir2.len(), 1);

        // Check file
        let file = match dir2.get("test_file.txt") {
            Some(Node::File(f)) => f,
            _ => panic!("Expected file node"),
        };

        assert_eq!(file.metadata.length, 200);
        assert_eq!(file.metadata.pieces_root.as_ref(), &root);
    }
}
