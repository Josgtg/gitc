use std::ffi::OsString;
use std::fmt::{format, Display};
use std::io::{BufRead, Cursor, Read, Write};
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{Context, Result, anyhow, bail};
use byteorder::WriteBytesExt;

use crate::fs::object::write_object;
use crate::hashing::{HASH_BYTE_LEN, Hash};
use crate::object::Object;
use crate::object::commit::TREE_STR;

const NULL_BYTE: u8 = b'\0';
const SPACE_BYTE: u8 = b' ';

/// Struct that represents a single tree entry in a tree object.
#[derive(Debug)]
pub struct TreeEntry {
    /// The mode is always stored as its octal representation
    pub mode: u32,
    pub path: PathBuf,
    pub hash: Hash,
}

impl Display for TreeEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}\t{}\t{}",
            self.mode,
            self.hash,
            self.path.to_string_lossy(),
        ))
    }
}

/// Represents a tree with a bit extra information (the subtrees linked to it)
pub struct TreeExt {
    pub path: PathBuf,
    pub tree: Object,
}

/// Will encode this tree object to a binary format, following the next layout:
///
/// "`tree {len}\0{entries}`"
///
/// Where `entries` have this format:
///
/// "`{mode} {filename}\0{hash}`"
///
/// # Errors
pub fn as_bytes(entries: &[TreeEntry]) -> Result<Rc<[u8]>> {
    // Cursor for tree entries
    let mut entries_bytes: Vec<u8> = Vec::new();
    for e in entries {
        entries_bytes.extend(format!("{} {}\0", e.mode, e.path.to_string_lossy()).as_bytes());
        entries_bytes.extend(e.hash.as_ref());
    }

    let mut header = format!("{} {}\0", Object::TREE_STRING, entries_bytes.len())
        .as_bytes()
        .to_vec();
    header.extend(entries_bytes);

    Ok(header.into())
}

pub fn from_bytes(bytes: &[u8]) -> Result<Object> {
    let mut cursor = Cursor::new(bytes);

    let mut kind_buf = Vec::new();
    cursor
        .read_until(SPACE_BYTE, &mut kind_buf)
        .context("could not read type")?;
    if kind_buf.pop() != Some(SPACE_BYTE) {
        bail!("expected space after object type");
    }

    let kind = String::from_utf8_lossy(&kind_buf);
    if kind != Object::TREE_STRING {
        bail!("object is not a tree, got: {}", kind)
    }

    let mut len_buf = Vec::new();
    cursor
        .read_until(NULL_BYTE, &mut len_buf)
        .context("could not read lenght")?;
    if len_buf.pop() != Some(NULL_BYTE) {
        bail!("expected null byte after data length")
    }

    let data_len: usize = String::from_utf8(len_buf)
        .context("failed to build string from object's decoded data length")?
        .parse()
        .map_err(|e| anyhow!("could not read data object lenght as a number: {:?}", e))?;

    let mut entries = Vec::new();
    let mut mode_buf: Vec<u8>;
    let mut path_buf: Vec<u8>;
    let mut hash_buf = [0; HASH_BYTE_LEN];
    let mut actual_len = 0;
    let mut bytes_read: usize;
    loop {
        // reading mode
        mode_buf = Vec::new();
        bytes_read = cursor
            .read_until(SPACE_BYTE, &mut mode_buf)
            .context("could not read tree entry mode")?;

        // If this returned 0, the file has ended
        if bytes_read == 0 {
            break;
        }

        if mode_buf.pop() != Some(SPACE_BYTE) {
            bail!("expected space after tree entry mode")
        }
        actual_len += mode_buf.len() + 1; // One for the space byte

        // reading path
        path_buf = Vec::new();
        cursor
            .read_until(NULL_BYTE, &mut path_buf)
            .context("could not read tree entry path")?;
        if path_buf.pop() != Some(NULL_BYTE) {
            bail!("expected null byte after tree entry path")
        }
        actual_len += path_buf.len() + 1; // One for the null byte

        // reading hash
        cursor
            .read_exact(&mut hash_buf)
            .context("could not read tree entry hash")?;
        actual_len += HASH_BYTE_LEN;

        // creating and adding tree entry
        entries.push(TreeEntry {
            mode: String::from_utf8_lossy(&mode_buf)
                .parse::<u32>()
                .context("could not get mode from bytes read (could not parse to a number)")?,
            path: PathBuf::from(OsString::from_vec(path_buf)),
            hash: Hash::from(hash_buf),
        });
    }

    if actual_len != data_len {
        bail!(
            "actual data len {} did not match object data len {}",
            actual_len,
            data_len
        )
    }

    Ok(Object::Tree { entries })
}

pub fn display(entries: &[TreeEntry]) -> String {
    let mut s = String::new();
    for e in entries {
        s.push_str(&e.to_string());
        s.push('\n');
    }
    s.pop();  // removing trailing newline
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashing::Hash;
    use crate::object::Object;
    use std::ffi::OsString;
    use std::str::FromStr;

    // Constants for test data
    const TEST_MODE_FILE: u32 = 0o100644;
    const TEST_MODE_EXECUTABLE: u32 = 0o100755;
    const TEST_MODE_DIR: u32 = 0o40755;
    const TEST_HASH_1: &str = "99ad2293829e9638b4dfeeb7bc405a4d140e84e3";
    const TEST_HASH_2: &str = "3e9713cc8320cc020e39b53566b2a34022608edc";
    const TEST_HASH_3: &str = "99800b85d3383e3a2fb45eb7d0066a4879a9dad0";
    const TEST_FILENAME_1: &str = "file1.txt";
    const TEST_FILENAME_2: &str = "script.sh";
    const TEST_FILENAME_3: &str = "subdir";
    const NULL_BYTE: u8 = b'\0';
    const HASH_SIZE: usize = HASH_BYTE_LEN;

    // Helper functions
    fn create_test_entry(mode: u32, path: &str, hash: &str) -> TreeEntry {
        TreeEntry {
            mode,
            path: PathBuf::from(path),
            hash: Hash::from_str(hash).unwrap(),
        }
    }

    fn create_test_entries() -> Vec<TreeEntry> {
        vec![
            create_test_entry(TEST_MODE_FILE, TEST_FILENAME_1, TEST_HASH_1),
            create_test_entry(TEST_MODE_EXECUTABLE, TEST_FILENAME_2, TEST_HASH_2),
            create_test_entry(TEST_MODE_DIR, TEST_FILENAME_3, TEST_HASH_3),
        ]
    }

    fn assert_contains_entry_data(bytes: &[u8], mode: u32, filename: &str, hash: &str) {
        let expected_entry = format!("{} {}\0", mode, filename);

        // Find the entry in the bytes
        let bytes_str = String::from_utf8_lossy(bytes);
        assert!(
            bytes_str.contains(&expected_entry[..expected_entry.len() - 1]),
            "Entry data not found in bytes: expected mode {} and filename {}",
            mode,
            filename
        );

        // Check that the hash is present (binary data, so we check bytes directly)
        let mut found_hash = false;
        let hash_bytes = Hash::from_str(hash).unwrap();
        for window in bytes.windows(HASH_SIZE) {
            if window == hash_bytes.as_ref() {
                found_hash = true;
                break;
            }
        }
        assert!(found_hash, "Hash not found in bytes");
    }

    #[test]
    fn test_as_bytes_empty_tree() {
        let entries = vec![];
        let result = as_bytes(&entries).unwrap();

        let expected = format!("tree 0\0");
        assert_eq!(expected.as_bytes(), result.as_ref());
    }

    #[test]
    fn test_as_bytes_single_entry() {
        let entries = vec![create_test_entry(
            TEST_MODE_FILE,
            TEST_FILENAME_1,
            TEST_HASH_1,
        )];
        let result = as_bytes(&entries).unwrap();

        // Check header
        assert!(result.starts_with(b"tree "));

        // Check that it contains the entry data
        assert_contains_entry_data(&result, TEST_MODE_FILE, TEST_FILENAME_1, &TEST_HASH_1);

        // Check structure: should have "tree {len}\0{data}"
        let null_pos = result.iter().position(|&b| b == NULL_BYTE).unwrap();
        let header = &result[..null_pos];
        let header_str = String::from_utf8_lossy(header);
        assert!(header_str.starts_with("tree "));
    }

    #[test]
    fn test_as_bytes_multiple_entries() {
        let entries = create_test_entries();
        let result = as_bytes(&entries).unwrap();

        // Check header
        assert!(result.starts_with(b"tree "));

        // Check that all entries are present
        assert_contains_entry_data(&result, TEST_MODE_FILE, TEST_FILENAME_1, &TEST_HASH_1);
        assert_contains_entry_data(&result, TEST_MODE_EXECUTABLE, TEST_FILENAME_2, &TEST_HASH_2);
        assert_contains_entry_data(&result, TEST_MODE_DIR, TEST_FILENAME_3, &TEST_HASH_3);
    }

    #[test]
    fn test_from_bytes_empty_tree() {
        let input = b"tree 0\0";
        let result = from_bytes(input).unwrap();

        match result {
            Object::Tree { entries } => {
                assert_eq!(0, entries.len());
            }
            _ => panic!("Expected Tree object"),
        }
    }

    #[test]
    fn test_from_bytes_single_entry() {
        let entries = vec![create_test_entry(
            TEST_MODE_FILE,
            TEST_FILENAME_1,
            TEST_HASH_1,
        )];
        let bytes = as_bytes(&entries).unwrap();
        let result = from_bytes(&bytes).unwrap();

        match result {
            Object::Tree {
                entries: parsed_entries,
            } => {
                assert_eq!(1, parsed_entries.len());
                assert_eq!(TEST_MODE_FILE, parsed_entries[0].mode);
                assert_eq!(OsString::from(TEST_FILENAME_1), parsed_entries[0].path);
                assert_eq!(Hash::from_str(TEST_HASH_1).unwrap(), parsed_entries[0].hash);
            }
            _ => panic!("Expected Tree object"),
        }
    }

    #[test]
    fn test_from_bytes_multiple_entries() {
        let entries = create_test_entries();
        let bytes = as_bytes(&entries).unwrap();
        let result = from_bytes(&bytes).unwrap();

        match result {
            Object::Tree {
                entries: parsed_entries,
            } => {
                assert_eq!(3, parsed_entries.len());

                assert_eq!(TEST_MODE_FILE, parsed_entries[0].mode);
                assert_eq!(OsString::from(TEST_FILENAME_1), parsed_entries[0].path);
                assert_eq!(Hash::from_str(TEST_HASH_1).unwrap(), parsed_entries[0].hash);

                assert_eq!(TEST_MODE_EXECUTABLE, parsed_entries[1].mode);
                assert_eq!(OsString::from(TEST_FILENAME_2), parsed_entries[1].path);
                assert_eq!(Hash::from_str(TEST_HASH_2).unwrap(), parsed_entries[1].hash);

                assert_eq!(TEST_MODE_DIR, parsed_entries[2].mode);
                assert_eq!(OsString::from(TEST_FILENAME_3), parsed_entries[2].path);
                assert_eq!(Hash::from_str(TEST_HASH_3).unwrap(), parsed_entries[2].hash);
            }
            _ => panic!("Expected Tree object"),
        }
    }

    #[test]
    fn test_roundtrip_consistency() {
        let original_entries = create_test_entries();
        let bytes = as_bytes(&original_entries).unwrap();
        let result = from_bytes(&bytes).unwrap();

        match result {
            Object::Tree {
                entries: parsed_entries,
            } => {
                assert_eq!(original_entries.len(), parsed_entries.len());

                for (original, parsed) in original_entries.iter().zip(parsed_entries.iter()) {
                    assert_eq!(original.mode, parsed.mode);
                    assert_eq!(original.path, parsed.path);
                    assert_eq!(original.hash.as_ref(), parsed.hash.as_ref());
                }
            }
            _ => panic!("Expected Tree object"),
        }
    }

    #[test]
    fn test_from_bytes_invalid_object_type() {
        let input = b"blob 0\0";
        let result = from_bytes(input);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("object is not a tree")
        );
    }

    #[test]
    fn test_from_bytes_missing_space_after_type() {
        let input = b"tree0\0";
        let result = from_bytes(input);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expected space after object type")
        );
    }

    #[test]
    fn test_from_bytes_missing_null_after_length() {
        let input = b"tree 0 ";
        let result = from_bytes(input);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expected null byte after data length")
        );
    }

    #[test]
    fn test_from_bytes_invalid_length() {
        let input = b"tree abc\0";
        let result = from_bytes(input);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("could not read data object lenght as a number")
        );
    }

    #[test]
    fn test_from_bytes_length_mismatch() {
        let mut input = Vec::from(b"tree 100\0100644 file.txt\0");
        input.extend(
            // Important to format the hash string as a hex encoded string
            Hash::from_str("1111111111111111111111111111111111111111")
                .unwrap()
                .as_ref(),
        );
        let result = from_bytes(input.as_ref());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("actual data len"));
    }

    #[test]
    fn test_from_bytes_missing_space_after_mode() {
        let input = b"tree 15\0100644filename\0";
        let result = from_bytes(input);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expected space after tree entry mode")
        );
    }

    #[test]
    fn test_from_bytes_missing_null_after_path() {
        let input = b"tree 15\0100644 filename ";
        let result = from_bytes(input);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expected null byte after tree entry path")
        );
    }

    #[test]
    fn test_from_bytes_incomplete_hash() {
        let input = b"tree 20\0100644 file\0short_hash";
        let result = from_bytes(input);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("could not read tree entry hash")
        );
    }

    #[test]
    fn test_from_bytes_invalid_mode() {
        let mut input = Vec::new();
        input.extend_from_slice(b"tree 32\0invalid file\0");
        input.extend_from_slice(TEST_HASH_1.as_bytes());

        let result = from_bytes(&input);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("could not get mode from bytes read")
        );
    }

    #[test]
    fn test_special_characters_in_filename() {
        let special_filename = "file-with.special_chars";
        let entries = vec![create_test_entry(
            TEST_MODE_FILE,
            special_filename,
            TEST_HASH_1,
        )];
        let bytes = as_bytes(&entries).unwrap();
        let result = from_bytes(&bytes).unwrap();

        match result {
            Object::Tree {
                entries: parsed_entries,
            } => {
                assert_eq!(1, parsed_entries.len());
                assert_eq!(OsString::from(special_filename), parsed_entries[0].path);
            }
            _ => panic!("Expected Tree object"),
        }
    }

    #[test]
    fn test_zero_mode() {
        let entries = vec![create_test_entry(0, "zero_mode_file", TEST_HASH_1)];
        let bytes = as_bytes(&entries).unwrap();
        let result = from_bytes(&bytes).unwrap();

        match result {
            Object::Tree {
                entries: parsed_entries,
            } => {
                assert_eq!(1, parsed_entries.len());
                assert_eq!(0, parsed_entries[0].mode);
            }
            _ => panic!("Expected Tree object"),
        }
    }
}
