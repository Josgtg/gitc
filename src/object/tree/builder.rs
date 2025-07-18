use std::collections::HashMap;
use std::path::PathBuf;

use crate::hashing::Hash;
use crate::object::Object;
use crate::utils;

use super::TreeEntry;

pub struct TreeBuilder {
    entries: Vec<TreeEntry>,
    /// Every entry on the hashmap represents a subtree, where the path is relative to it's parent
    /// tree's path.
    subtrees: Option<HashMap<PathBuf, TreeBuilder>>,
}

impl TreeBuilder {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            subtrees: None,
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
            if self.subtrees.is_none() {
                self.subtrees = Some(HashMap::new());
            }

            let subtrees = self.subtrees.as_mut().unwrap();

            // Stripping the root from a path, this is done so the subtrees don't store the
            // directory they are supposed to represent.
            let (option_root, stripped_path) = utils::path::strip_root(path.clone());
            let root = option_root.expect("path should have a parent since it is a dir");

            // Updating the corresponding subtree or adding a new one if a subtree for this path
            // does not exist.
            let tree = subtrees.get_mut(&root);
            if let Some(t) = tree {
                t.add_object(mode, stripped_path, hash);
            } else {
                let mut tree = TreeBuilder::new();
                tree.add_object(mode, stripped_path, hash);
                subtrees.insert(root, tree);
            }
        } else {
            self.entries.push(TreeEntry { mode, path, hash });
        }
    }

    /// Returns the entries for this tree builder, without turning it into a tree object.
    fn build_entries(self) -> Vec<TreeEntry> {
        let mut entries = self.entries;
        if let Some(subtrees) = self.subtrees {
            let mut subentries;
            for (path, tree) in subtrees {
                // Adds subdirectory to the start of every tree entry's path inside of this subtree
                subentries = tree.build_entries();
                for se in subentries.iter_mut() {
                    se.path = path.join(std::mem::take(&mut se.path));
                }
                entries.extend(subentries);
            }
        }
        entries
    }

    /// Gets all the entries from this tree builder, consuming it and returning a tree object
    /// containing said entries.
    pub fn build(self) -> Object {
        Object::Tree {
            entries: self.build_entries(),
        }
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
    const TEST_MODE_DIR: u32 = 0o040000;
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
    const TEST_HASH_4: [u8; 20] = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14,
    ];

    // Helper functions
    fn create_hash(bytes: [u8; 20]) -> Hash {
        Hash::from(bytes)
    }

    fn create_file_path(path: &str) -> PathBuf {
        PathBuf::from(path)
    }

    fn create_dir_path(path: &str) -> PathBuf {
        let mut pb = PathBuf::from(path);
        // Ensure it's treated as a directory by adding a trailing separator if needed
        if !pb.as_os_str().to_string_lossy().ends_with('/') {
            pb = pb.join("");
        }
        pb
    }

    fn assert_entry_exists(
        entries: &[TreeEntry],
        expected_path: &str,
        expected_mode: u32,
        expected_hash: Hash,
    ) {
        let found = entries.iter().find(|entry| {
            entry.path == PathBuf::from(expected_path)
                && entry.mode == expected_mode
                && entry.hash == expected_hash
        });
        assert!(
            found.is_some(),
            "Expected entry not found: path={}, mode={}, hash={:?}",
            expected_path,
            expected_mode,
            expected_hash
        );
    }

    fn count_entries_with_path_prefix(entries: &[TreeEntry], prefix: &str) -> usize {
        entries
            .iter()
            .filter(|entry| entry.path.starts_with(prefix))
            .count()
    }

    #[test]
    fn test_new_tree_builder() {
        let builder = TreeBuilder::new();
        assert_eq!(builder.entries.len(), 0);
        assert!(builder.subtrees.is_none());
    }

    #[test]
    fn test_add_single_file() {
        let mut builder = TreeBuilder::new();
        let hash = create_hash(TEST_HASH_1);
        let path = create_file_path("file1.txt");

        builder.add_object(TEST_MODE_FILE, path.clone(), hash.clone());

        assert_eq!(builder.entries.len(), 1);
        assert_eq!(builder.entries[0].mode, TEST_MODE_FILE);
        assert_eq!(builder.entries[0].path, path);
        assert_eq!(builder.entries[0].hash, hash);
        assert!(builder.subtrees.is_none());
    }

    #[test]
    fn test_add_multiple_files() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);
        let path1 = create_file_path("file1.txt");
        let path2 = create_file_path("file2.txt");

        builder.add_object(TEST_MODE_FILE, path1.clone(), hash1);
        builder.add_object(TEST_MODE_EXECUTABLE, path2.clone(), hash2);

        assert_eq!(builder.entries.len(), 2);
        assert_eq!(builder.entries[0].path, path1);
        assert_eq!(builder.entries[1].path, path2);
        assert!(builder.subtrees.is_none());
    }

    #[test]
    fn test_add_file_in_directory() {
        let mut builder = TreeBuilder::new();
        let hash = create_hash(TEST_HASH_1);
        let file_path = create_dir_path("src/main.rs");
        dbg!(file_path.is_dir());
        builder.add_object(TEST_MODE_FILE, file_path, hash);

        assert_eq!(builder.entries.len(), 0);
        assert!(builder.subtrees.is_some());

        let subtrees = builder.subtrees.as_ref().unwrap();
        assert_eq!(subtrees.len(), 1);
        assert!(subtrees.contains_key(&PathBuf::from("src")));

        let src_subtree = subtrees.get(&PathBuf::from("src")).unwrap();
        assert_eq!(src_subtree.entries.len(), 1);
        assert_eq!(src_subtree.entries[0].path, PathBuf::from("main.rs"));
    }

    #[test]
    fn test_add_multiple_files_same_directory() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);
        let file1_path = create_dir_path("src/main.rs");
        let file2_path = create_dir_path("src/lib.rs");

        builder.add_object(TEST_MODE_FILE, file1_path, hash1);
        builder.add_object(TEST_MODE_FILE, file2_path, hash2);

        assert_eq!(builder.entries.len(), 0);
        assert!(builder.subtrees.is_some());

        let subtrees = builder.subtrees.as_ref().unwrap();
        assert_eq!(subtrees.len(), 1);

        let src_subtree = subtrees.get(&PathBuf::from("src")).unwrap();
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
        let file1_path = create_dir_path("src/main.rs");
        let file2_path = create_dir_path("tests/test.rs");

        builder.add_object(TEST_MODE_FILE, file1_path, hash1);
        builder.add_object(TEST_MODE_FILE, file2_path, hash2);

        assert_eq!(builder.entries.len(), 0);
        assert!(builder.subtrees.is_some());

        let subtrees = builder.subtrees.as_ref().unwrap();
        assert_eq!(subtrees.len(), 2);
        assert!(subtrees.contains_key(&PathBuf::from("src")));
        assert!(subtrees.contains_key(&PathBuf::from("tests")));

        let src_subtree = subtrees.get(&PathBuf::from("src")).unwrap();
        assert_eq!(src_subtree.entries.len(), 1);
        assert_eq!(src_subtree.entries[0].path, PathBuf::from("main.rs"));

        let tests_subtree = subtrees.get(&PathBuf::from("tests")).unwrap();
        assert_eq!(tests_subtree.entries.len(), 1);
        assert_eq!(tests_subtree.entries[0].path, PathBuf::from("test.rs"));
    }

    #[test]
    fn test_nested_directories() {
        let mut builder = TreeBuilder::new();
        let hash = create_hash(TEST_HASH_1);
        let nested_path = create_dir_path("src/utils/helper.rs");

        builder.add_object(TEST_MODE_FILE, nested_path, hash);

        assert_eq!(builder.entries.len(), 0);
        assert!(builder.subtrees.is_some());

        let subtrees = builder.subtrees.as_ref().unwrap();
        assert_eq!(subtrees.len(), 1);
        assert!(subtrees.contains_key(&PathBuf::from("src")));

        let src_subtree = subtrees.get(&PathBuf::from("src")).unwrap();
        assert_eq!(src_subtree.entries.len(), 0);
        assert!(src_subtree.subtrees.is_some());

        let src_subtrees = src_subtree.subtrees.as_ref().unwrap();
        assert_eq!(src_subtrees.len(), 1);
        assert!(src_subtrees.contains_key(&PathBuf::from("utils")));

        let utils_subtree = src_subtrees.get(&PathBuf::from("utils")).unwrap();
        assert_eq!(utils_subtree.entries.len(), 1);
        assert_eq!(utils_subtree.entries[0].path, PathBuf::from("helper.rs"));
    }

    #[test]
    fn test_mixed_files_and_directories() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);
        let hash3 = create_hash(TEST_HASH_3);

        let root_file = create_file_path("README.md");
        let src_file = create_dir_path("src/main.rs");
        let test_file = create_dir_path("tests/test.rs");

        builder.add_object(TEST_MODE_FILE, root_file.clone(), hash1);
        builder.add_object(TEST_MODE_FILE, src_file, hash2);
        builder.add_object(TEST_MODE_FILE, test_file, hash3);

        // Should have one root-level file
        assert_eq!(builder.entries.len(), 1);
        assert_eq!(builder.entries[0].path, root_file);

        // Should have two subtrees
        assert!(builder.subtrees.is_some());
        let subtrees = builder.subtrees.as_ref().unwrap();
        assert_eq!(subtrees.len(), 2);
        assert!(subtrees.contains_key(&PathBuf::from("src")));
        assert!(subtrees.contains_key(&PathBuf::from("tests")));
    }

    #[test]
    fn test_build_entries_empty() {
        let builder = TreeBuilder::new();
        let entries = builder.build_entries();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_build_entries_root_files_only() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);

        builder.add_object(TEST_MODE_FILE, create_file_path("file1.txt"), hash1.clone());
        builder.add_object(TEST_MODE_FILE, create_file_path("file2.txt"), hash2.clone());

        let entries = builder.build_entries();
        assert_eq!(entries.len(), 2);

        assert_entry_exists(&entries, "file1.txt", TEST_MODE_FILE, hash1);
        assert_entry_exists(&entries, "file2.txt", TEST_MODE_FILE, hash2);
    }

    #[test]
    fn test_build_entries_with_subtrees() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);

        builder.add_object(TEST_MODE_FILE, create_file_path("README.md"), hash1.clone());
        builder.add_object(
            TEST_MODE_FILE,
            create_dir_path("src/main.rs"),
            hash2.clone(),
        );

        let entries = builder.build_entries();
        assert_eq!(entries.len(), 2);

        assert_entry_exists(&entries, "README.md", TEST_MODE_FILE, hash1);
        assert_entry_exists(&entries, "src/main.rs", TEST_MODE_FILE, hash2);
    }

    #[test]
    fn test_build_entries_nested_subtrees() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);
        let hash3 = create_hash(TEST_HASH_3);

        builder.add_object(TEST_MODE_FILE, create_file_path("root.txt"), hash1.clone());
        builder.add_object(
            TEST_MODE_FILE,
            create_dir_path("src/main.rs"),
            hash2.clone(),
        );
        builder.add_object(
            TEST_MODE_FILE,
            create_dir_path("src/utils/helper.rs"),
            hash3.clone(),
        );

        let entries = builder.build_entries();
        assert_eq!(entries.len(), 3);

        assert_entry_exists(&entries, "root.txt", TEST_MODE_FILE, hash1);
        assert_entry_exists(&entries, "src/main.rs", TEST_MODE_FILE, hash2);
        assert_entry_exists(&entries, "src/utils/helper.rs", TEST_MODE_FILE, hash3);
    }

    #[test]
    fn test_build_entries_multiple_subtrees() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);
        let hash3 = create_hash(TEST_HASH_3);
        let hash4 = create_hash(TEST_HASH_4);

        builder.add_object(
            TEST_MODE_FILE,
            create_dir_path("src/main.rs"),
            hash1.clone(),
        );
        builder.add_object(TEST_MODE_FILE, create_dir_path("src/lib.rs"), hash2.clone());
        builder.add_object(
            TEST_MODE_FILE,
            create_dir_path("tests/test1.rs"),
            hash3.clone(),
        );
        builder.add_object(
            TEST_MODE_FILE,
            create_dir_path("tests/test2.rs"),
            hash4.clone(),
        );

        let entries = builder.build_entries();
        assert_eq!(entries.len(), 4);

        assert_entry_exists(&entries, "src/main.rs", TEST_MODE_FILE, hash1);
        assert_entry_exists(&entries, "src/lib.rs", TEST_MODE_FILE, hash2);
        assert_entry_exists(&entries, "tests/test1.rs", TEST_MODE_FILE, hash3);
        assert_entry_exists(&entries, "tests/test2.rs", TEST_MODE_FILE, hash4);

        // Verify correct grouping
        assert_eq!(count_entries_with_path_prefix(&entries, "src/"), 2);
        assert_eq!(count_entries_with_path_prefix(&entries, "tests/"), 2);
    }

    #[test]
    fn test_build_object_empty() {
        let builder = TreeBuilder::new();
        let obj = builder.build();

        match obj {
            Object::Tree { entries } => {
                assert_eq!(entries.len(), 0);
            }
            _ => panic!("Expected Tree object"),
        }
    }

    #[test]
    fn test_build_object_with_files() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);

        builder.add_object(TEST_MODE_FILE, create_file_path("file1.txt"), hash1.clone());
        builder.add_object(
            TEST_MODE_FILE,
            create_dir_path("src/main.rs"),
            hash2.clone(),
        );

        let obj = builder.build();

        match obj {
            Object::Tree { entries } => {
                assert_eq!(entries.len(), 2);
                assert_entry_exists(&entries, "file1.txt", TEST_MODE_FILE, hash1);
                assert_entry_exists(&entries, "src/main.rs", TEST_MODE_FILE, hash2);
            }
            _ => panic!("Expected Tree object"),
        }
    }

    #[test]
    fn test_path_reconstruction_preserves_structure() {
        let mut builder = TreeBuilder::new();
        let hash1 = create_hash(TEST_HASH_1);
        let hash2 = create_hash(TEST_HASH_2);
        let hash3 = create_hash(TEST_HASH_3);

        let original_paths = vec!["src/main.rs", "src/lib.rs", "tests/integration/test.rs"];

        builder.add_object(TEST_MODE_FILE, create_dir_path(original_paths[0]), hash1);
        builder.add_object(TEST_MODE_FILE, create_dir_path(original_paths[1]), hash2);
        builder.add_object(TEST_MODE_FILE, create_dir_path(original_paths[2]), hash3);

        let entries = builder.build_entries();
        assert_eq!(entries.len(), 3);

        // Verify all original paths are preserved
        for original_path in original_paths {
            let found = entries
                .iter()
                .any(|entry| entry.path == PathBuf::from(original_path));
            assert!(found, "Path '{}' not found in built entries", original_path);
        }
    }

    #[test]
    fn test_single_character_directory_names() {
        let mut builder = TreeBuilder::new();
        let hash = create_hash(TEST_HASH_1);

        builder.add_object(TEST_MODE_FILE, create_dir_path("a/b/c.txt"), hash.clone());

        let entries = builder.build_entries();
        assert_eq!(entries.len(), 1);
        assert_entry_exists(&entries, "a/b/c.txt", TEST_MODE_FILE, hash);
    }

    #[test]
    fn test_deep_nesting() {
        let mut builder = TreeBuilder::new();
        let hash = create_hash(TEST_HASH_1);

        let deep_path = "level1/level2/level3/level4/level5/file.txt";
        builder.add_object(TEST_MODE_FILE, create_dir_path(deep_path), hash.clone());

        let entries = builder.build_entries();
        assert_eq!(entries.len(), 1);
        assert_entry_exists(&entries, deep_path, TEST_MODE_FILE, hash);
    }
}
