use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::fs::object::write_object;
use crate::hashing::Hash;
use crate::object::Object;
use crate::utils;
use crate::utils::nums::as_octal;

use super::TreeEntry;
use super::tree::TreeExt;

const DEFAULT_DIR_MODE: u32 = 0o40000;

#[derive(Debug)]
pub struct TreeBuilder {
    entries: Vec<TreeEntry>,
    /// Every entry on the hashmap represents a subtree, where the path is relative to it's parent
    /// tree's path.
    subtrees: HashMap<PathBuf, TreeBuilder>,
}

impl TreeBuilder {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            subtrees: HashMap::new(),
        }
    }

    /// Adds an object to this tree. It has two behaviors:
    /// 1. If the file is not a directory, it is just added to the current tree.
    /// 2. If the file is a directory, a new tree is created and the object is added there instead.
    pub fn add_object(&mut self, mode: u32, path: PathBuf, hash: Hash) {
        if path.is_dir() {
            // There is no use on adding just a directory
            return;
        }

        if path.components().count() > 1 {
            // If there is more than one component, it means there is a subdirectory in the path

            // Stripping the root from a path, this is done so the subtrees don't store the
            // directory they are supposed to represent.
            let (option_root, stripped_path) = utils::path::strip_root(path.clone());
            let root = option_root.expect("path should have a parent since it is a dir");

            // Updating the corresponding subtree or adding a new one if a subtree for this path
            // does not exist.
            let tree = self.subtrees.get_mut(&root);
            if let Some(t) = tree {
                t.add_object(mode, stripped_path, hash);
            } else {
                let mut tree = TreeBuilder::new();
                tree.add_object(mode, stripped_path, hash);
                self.subtrees.insert(root, tree);
            }
        } else {
            self.entries.push(TreeEntry {
                mode: as_octal(mode),
                path,
                hash,
            });
        }
    }

    /// Builds the tree and subsequent subtrees, assgining `path` to this tree.
    ///
    /// If `write` is set, the hash would be obtained by writing the object to the object dir, if
    /// it's not, then the hash will just be computed from scratch.
    fn build_as_subtree(mut self, subdir: PathBuf, write: bool) -> Result<TreeExt> {
        let mut subtrees: Vec<TreeExt> = Vec::new();
        for (p, t) in self.subtrees.into_iter() {
            subtrees.push(
                t.build_as_subtree(p, write)
                    .context("could not build tree")?,
            );
        }

        // Updating undefined hashes
        let mut hash: Hash;
        let mut mode: u32;
        for subt in subtrees {
            hash = if write {
                write_object(&subt.tree).context("could not write subtree")?
            } else {
                subt.tree.hash().context("could not hash tree")?
            };
            mode = if let Ok(m) = subt.path.metadata() {
                m.mode()
            } else {
                // Mode here should always be a directory's mode anyways so it can be hardcoded
                DEFAULT_DIR_MODE
            };
            // Adding entry for this subtree in the main tree
            self.entries.push(TreeEntry {
                mode: as_octal(mode),
                path: subt.path,
                hash,
            });
        }

        Ok(TreeExt {
            path: subdir,
            tree: Object::Tree {
                entries: self.entries,
            },
        })
    }

    /// Gets all the entries from this tree builder, consuming it and returning a tree object
    /// containing said entries.
    ///
    /// If you want to use this object to immediately write it to the objects directory, use the
    /// `build_and_write` method instead.
    ///
    /// # Errors
    ///
    /// This function can fail if a hash for an entry could not be computed.
    #[allow(unused)]
    pub fn build(self) -> Result<TreeExt> {
        self.build_as_subtree(PathBuf::new(), false) // Since it's the root one, the path remains empty
    }

    /// Gets all the entries from this tree builder, consuming it and building a tree form the
    /// entries, immediately writing it to the objects directory.
    ///
    /// The important thing here is that this avoids computing a hash from scratch for the subtrees
    /// since it is obtained when writing the object file.
    ///
    /// # Errors
    ///
    /// This function can fail if it was not possible to write the object or build it in the first
    /// place.
    pub fn build_and_write(self) -> Result<Hash> {
        let treext = self
            .build_as_subtree(PathBuf::new(), true)
            .context("could not write subtrees")?;
        write_object(&treext.tree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashing::Hash;
    use crate::object::Object;
    use std::path::PathBuf;

    // Constants for test data
    const TEST_MODE_FILE: u32 = 0o100644;
    const TEST_MODE_EXECUTABLE: u32 = 0o100755;
    const TEST_HASH_1: [u8; 20] = [
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde,
        0xf0, 0x12, 0x34, 0x56, 0x78,
    ];
    const TEST_HASH_2: [u8; 20] = [
        0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78,
        0x9a, 0xbc, 0xde, 0xf0, 0x12,
    ];
    const TEST_HASH_3: [u8; 20] = [
        0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11,
        0x00, 0xff, 0xee, 0xdd, 0xcc,
    ];

    // Helper functions
    fn create_hash(bytes: [u8; 20]) -> Hash {
        Hash::from(bytes)
    }

    fn assert_entry_exists(
        entries: &[TreeEntry],
        expected_path: &str,
        expected_mode: u32,
        expected_hash: &Hash,
    ) {
        let expected_mode_octal = as_octal(expected_mode);
        let found = entries.iter().find(|entry| {
            entry.path == PathBuf::from(expected_path)
                && entry.mode == expected_mode_octal
                && entry.hash == *expected_hash
        });
        assert!(
            found.is_some(),
            "Expected entry not found: path={}, mode={}",
            expected_path,
            expected_mode_octal
        );
    }

    #[test]
    fn test_new_tree_builder() {
        let builder = TreeBuilder::new();
        assert_eq!(builder.entries.len(), 0);
        assert_eq!(builder.subtrees.len(), 0);
    }

    #[test]
    fn test_add_single_file() {
        let mut builder = TreeBuilder::new();
        let hash = create_hash(TEST_HASH_1);
        let path = PathBuf::from("file1.txt");

        builder.add_object(TEST_MODE_FILE, path.clone(), hash.clone());

        assert_eq!(builder.entries.len(), 1);
        assert_eq!(builder.entries[0].mode, as_octal(TEST_MODE_FILE));
        assert_eq!(builder.entries[0].path, path);
        assert_eq!(builder.entries[0].hash, hash);
        assert_eq!(builder.subtrees.len(), 0);
    }

    #[test]
    fn test_add_multiple_root_files() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);

        builder.add_object(TEST_MODE_FILE, PathBuf::from("file1.txt"), hash1);
        builder.add_object(TEST_MODE_EXECUTABLE, PathBuf::from("file2.txt"), hash2);

        assert_eq!(builder.entries.len(), 2);
        assert_eq!(builder.subtrees.len(), 0);
    }

    #[test]
    fn test_add_directory_ignored() {
        let mut builder = TreeBuilder::new();
        let hash = create_hash(TEST_HASH_1);
        let dir_path = PathBuf::from("src"); // Directory path

        builder.add_object(TEST_MODE_FILE, dir_path, hash);

        // Directory should be ignored
        assert_eq!(builder.entries.len(), 0);
        assert_eq!(builder.subtrees.len(), 0);
    }

    #[test]
    fn test_add_file_with_subdirectory() {
        let mut builder = TreeBuilder::new();
        let hash = create_hash(TEST_HASH_1);
        let file_path = PathBuf::from("src/main.rs");

        builder.add_object(TEST_MODE_FILE, file_path, hash.clone());

        assert_eq!(builder.entries.len(), 0);
        assert_eq!(builder.subtrees.len(), 1);
        assert!(builder.subtrees.contains_key(&PathBuf::from("src")));

        let src_subtree = builder.subtrees.get(&PathBuf::from("src")).unwrap();
        assert_eq!(src_subtree.entries.len(), 1);
        assert_eq!(src_subtree.entries[0].path, PathBuf::from("main.rs"));
        assert_eq!(src_subtree.entries[0].hash, hash);
    }

    #[test]
    fn test_add_multiple_files_same_directory() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);

        builder.add_object(TEST_MODE_FILE, PathBuf::from("src/main.rs"), hash1);
        builder.add_object(TEST_MODE_FILE, PathBuf::from("src/lib.rs"), hash2);

        assert_eq!(builder.entries.len(), 0);
        assert_eq!(builder.subtrees.len(), 1);

        let src_subtree = builder.subtrees.get(&PathBuf::from("src")).unwrap();
        assert_eq!(src_subtree.entries.len(), 2);

        let paths: Vec<_> = src_subtree.entries.iter().map(|e| &e.path).collect();
        assert!(paths.contains(&&PathBuf::from("main.rs")));
        assert!(paths.contains(&&PathBuf::from("lib.rs")));
    }

    #[test]
    fn test_add_files_different_directories() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);

        builder.add_object(TEST_MODE_FILE, PathBuf::from("src/main.rs"), hash1);
        builder.add_object(TEST_MODE_FILE, PathBuf::from("tests/test.rs"), hash2);

        assert_eq!(builder.entries.len(), 0);
        assert_eq!(builder.subtrees.len(), 2);
        assert!(builder.subtrees.contains_key(&PathBuf::from("src")));
        assert!(builder.subtrees.contains_key(&PathBuf::from("tests")));
    }

    #[test]
    fn test_nested_directories() {
        let mut builder = TreeBuilder::new();
        let hash = create_hash(TEST_HASH_1);

        builder.add_object(
            TEST_MODE_FILE,
            PathBuf::from("src/utils/helper.rs"),
            hash.clone(),
        );

        assert_eq!(builder.entries.len(), 0);
        assert_eq!(builder.subtrees.len(), 1);
        assert!(builder.subtrees.contains_key(&PathBuf::from("src")));

        let src_subtree = builder.subtrees.get(&PathBuf::from("src")).unwrap();
        assert_eq!(src_subtree.entries.len(), 0);
        assert_eq!(src_subtree.subtrees.len(), 1);
        assert!(src_subtree.subtrees.contains_key(&PathBuf::from("utils")));

        let utils_subtree = src_subtree.subtrees.get(&PathBuf::from("utils")).unwrap();
        assert_eq!(utils_subtree.entries.len(), 1);
        assert_eq!(utils_subtree.entries[0].path, PathBuf::from("helper.rs"));
        assert_eq!(utils_subtree.entries[0].hash, hash);
    }

    #[test]
    fn test_mixed_root_and_subdirectory_files() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);
        let hash3 = create_hash(TEST_HASH_3);

        builder.add_object(TEST_MODE_FILE, PathBuf::from("README.md"), hash1);
        builder.add_object(TEST_MODE_FILE, PathBuf::from("src/main.rs"), hash2);
        builder.add_object(TEST_MODE_FILE, PathBuf::from("tests/test.rs"), hash3);

        // Should have one root-level file
        assert_eq!(builder.entries.len(), 1);
        assert_eq!(builder.entries[0].path, PathBuf::from("README.md"));

        // Should have two subtrees
        assert_eq!(builder.subtrees.len(), 2);
        assert!(builder.subtrees.contains_key(&PathBuf::from("src")));
        assert!(builder.subtrees.contains_key(&PathBuf::from("tests")));
    }

    #[test]
    fn test_build_empty_tree() {
        let builder = TreeBuilder::new();
        let result = builder.build().unwrap();

        assert_eq!(result.path, PathBuf::new());

        match result.tree {
            Object::Tree { entries } => {
                assert_eq!(entries.len(), 0);
            }
            _ => panic!("Expected Tree object"),
        }
    }

    #[test]
    fn test_build_root_files_only() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);

        builder.add_object(TEST_MODE_FILE, PathBuf::from("file1.txt"), hash1.clone());
        builder.add_object(TEST_MODE_FILE, PathBuf::from("file2.txt"), hash2.clone());

        let result = builder.build().unwrap();

        assert_eq!(result.path, PathBuf::new());

        match result.tree {
            Object::Tree { entries } => {
                assert_eq!(entries.len(), 2);
                assert_entry_exists(&entries, "file1.txt", TEST_MODE_FILE, &hash1);
                assert_entry_exists(&entries, "file2.txt", TEST_MODE_FILE, &hash2);
            }
            _ => panic!("Expected Tree object"),
        }
    }

    #[test]
    fn test_build_with_subtrees() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);

        builder.add_object(TEST_MODE_FILE, PathBuf::from("README.md"), hash1.clone());
        builder.add_object(TEST_MODE_FILE, PathBuf::from("src/main.rs"), hash2.clone());

        let entries = &builder.subtrees.get(&PathBuf::from("src")).unwrap().entries;
        assert_entry_exists(&entries, "main.rs", TEST_MODE_FILE, &hash2);

        let result = builder.build().unwrap();

        assert_eq!(result.path, PathBuf::new());

        // Check that root tree has entries for both the file and the subtree
        match result.tree {
            Object::Tree { entries } => {
                assert_eq!(entries.len(), 2); // README.md + src/ directory entry
                assert_entry_exists(&entries, "README.md", TEST_MODE_FILE, &hash1);
                // The src directory entry should be present (added by build_as_subtree)
                let src_entry = entries.iter().find(|e| e.path == PathBuf::from("src"));
                assert!(src_entry.is_some(), "src directory entry not found");
            }
            _ => panic!("Expected Tree object"),
        }
    }

    #[test]
    fn test_build_nested_structure() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);

        builder.add_object(
            TEST_MODE_FILE,
            PathBuf::from("src/utils/helper.rs"),
            hash1.clone(),
        );
        builder.add_object(TEST_MODE_FILE, PathBuf::from("src/main.rs"), hash2);
        assert_eq!(builder.subtrees.len(), 1);

        // Check src subtree
        let src_subtree = &builder.subtrees.get(&PathBuf::from("src")).unwrap();
        assert_eq!(src_subtree.subtrees.len(), 1);

        // Check utils subtree
        let utils_subtree = &src_subtree.subtrees.get(&PathBuf::from("utils")).unwrap();

        assert_entry_exists(&utils_subtree.entries, "helper.rs", TEST_MODE_FILE, &hash1);
    }

    #[test]
    fn test_single_component_vs_multiple_components() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);

        // Single component - should go to entries
        builder.add_object(TEST_MODE_FILE, PathBuf::from("root.txt"), hash1);

        // Multiple components - should create subtree
        builder.add_object(TEST_MODE_FILE, PathBuf::from("dir/file.txt"), hash2);

        assert_eq!(builder.entries.len(), 1);
        assert_eq!(builder.subtrees.len(), 1);

        assert_eq!(builder.entries[0].path, PathBuf::from("root.txt"));
        assert!(builder.subtrees.contains_key(&PathBuf::from("dir")));
    }
}
