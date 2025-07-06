use std::ffi::OsString;
use std::io::{BufRead, Cursor, Read, Write};
use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use std::rc::Rc;

use anyhow::{anyhow, bail, Context, Result};
use byteorder::WriteBytesExt;

use crate::hashing::Hash;

use super::Object;

#[derive(Debug)]
pub struct TreeEntry {
    mode: u32,
    path: OsString,
    hash: Hash,
}

pub struct TreeBuilder {
    entries: Vec<TreeEntry>,
}

impl TreeBuilder {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add_object(&mut self, mode: u32, path: &Path, hash: &Hash) {
        self.entries.push(TreeEntry {
            mode,
            path: path.into(),
            hash: hash.clone(),
        })
    }

    pub fn build(self) -> Object {
        Object::Tree {
            entries: self.entries,
        }
    }
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
    let mut entries_cursor = Cursor::new(Vec::new());
    for e in entries {
        // mode is stored as an ascii, octal formatted number
        entries_cursor
            .write_all(format!("{:o}", e.mode).as_bytes())
            .context("could not write tree entry mode")?;
        entries_cursor.write_u8(b' ')?;
        entries_cursor
            .write_all(e.path.as_encoded_bytes())
            .context("could not write tree entry path")?;
        entries_cursor.write_u8(b'\0')?;
        entries_cursor
            .write_all(e.hash.as_ref())
            .context("could not write tree entry hash")?;
    }
    let entries_bytes = entries_cursor.into_inner();

    // Cursor for the whole tree
    let mut cursor = Cursor::new(Vec::new());
    cursor
        .write_all(Object::TREE_STRING.as_bytes())
        .context("could not write tree object name")?;
    cursor.write_u8(b' ')?;
    cursor
        .write_all(entries_bytes.len().to_string().as_bytes())
        .context("could not write data length")?;
    cursor.write_u8(b'\0')?;
    cursor
        .write_all(entries_bytes.as_ref())
        .context("could not write tree entries")?;

    Ok(cursor.into_inner().into())
}

pub fn from_bytes(bytes: &[u8]) -> Result<Object> {
    let mut cursor = Cursor::new(bytes);

    let mut kind_buf = Vec::new();
    cursor
        .read_until(b' ', &mut kind_buf)
        .context("could not read type")?;
    if kind_buf.pop() != Some(b' ') {
        bail!("expected space after object type");
    }

    let kind = String::from_utf8_lossy(&kind_buf);
    if kind != Object::TREE_STRING {
        bail!("object is not a tree, got: {}", kind)
    }

    let mut len_buf = Vec::new();
    cursor
        .read_until(b'\0', &mut len_buf)
        .context("could not read lenght")?;
    if len_buf.pop() != Some(b'\0') {
        bail!("expected null byte after data length")
    }

    let data_len: usize = String::from_utf8(len_buf)
        .context("failed to build string from object's decoded data length")?
        .parse()
        .map_err(|e| anyhow!("could not read data object lenght as a number: {:?}", e))?;

    let mut entries = Vec::new();
    let mut mode_buf: Vec<u8>;
    let mut path_buf: Vec<u8>;
    let mut hash_buf = [0; 20];
    let mut actual_len = 0;
    let mut bytes_read: usize;
    loop {
        mode_buf = Vec::new();
        bytes_read = cursor
            .read_until(b' ', &mut mode_buf)
            .context("could not read tree entry mode")?;
        // If this returned 0, the file has ended
        if bytes_read == 0 {
            break;
        }
        if mode_buf.pop() != Some(b' ') {
            bail!("expected space after tree entry mode")
        }
        actual_len += mode_buf.len();

        path_buf = Vec::new();
        cursor
            .read_until(b'\0', &mut path_buf)
            .context("could not read tree entry path")?;
        if path_buf.pop() != Some(b'\0') {
            bail!("expected null byte after tree entry path")
        }
        actual_len += path_buf.len();

        cursor
            .read_exact(&mut hash_buf)
            .context("could not read tree entry hash")?;
        actual_len += 20;

        entries.push(TreeEntry {
            mode: String::from_utf8_lossy(&mode_buf)
                .parse()
                .context("could not get mode from bytes read (could not parse to a number)")?,
            path: OsString::from_vec(path_buf),
            hash: Hash::from(hash_buf),
        })
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
