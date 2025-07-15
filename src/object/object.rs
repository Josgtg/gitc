use anyhow::{Context, Result, bail};
use std::io::{BufRead, Cursor};
use std::rc::Rc;

use crate::byteable::Byteable;
use crate::hashing::Hash;

use super::commit::CommitUser;
use super::tree::TreeEntry;

/// Represents the different type of objects there can be: Blobs, Commits and Trees, with methods
/// for byte encoding and decoding.
#[derive(Debug)]
pub enum Object {
    Blob {
        data: Rc<[u8]>,
    },
    Tree {
        entries: Vec<TreeEntry>,
    },
    Commit {
        tree: Hash,
        parent: Option<Hash>,
        author: CommitUser,
        committer: CommitUser,
        message: Rc<str>,
    },
}

impl Object {
    pub const BLOB_STRING: &str = "blob";
    pub const TREE_STRING: &str = "tree";
    pub const COMMIT_STRING: &str = "commit";
}

impl std::fmt::Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Blob { .. } => Object::BLOB_STRING,
            Self::Tree { .. } => Object::TREE_STRING,
            Self::Commit { .. } => Object::COMMIT_STRING,
        })
    }
}

impl Byteable for Object {
    fn as_bytes(&self) -> Result<Rc<[u8]>> {
        match self {
            Object::Blob { data } => super::blob::as_bytes(data),
            Object::Tree { entries } => super::tree::as_bytes(entries),
            Object::Commit {
                tree,
                parent,
                author,
                committer: commiter,
                message,
            } => super::commit::as_bytes(tree, parent.as_ref(), author, commiter, message),
        }
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Getting object type
        let mut kind_buffer = Vec::new();
        Cursor::new(bytes)
            .read_until(b' ', &mut kind_buffer)
            .context("could not get object type")?;

        let kind = String::from_utf8_lossy(&kind_buffer);
        match kind.as_ref() {
            Object::BLOB_STRING => super::blob::from_bytes(bytes),
            Object::TREE_STRING => super::tree::from_bytes(bytes),
            Object::COMMIT_STRING => super::commit::from_bytes(bytes),
            _ => bail!("object did not have a valid type, got: {}", kind),
        }
    }
}
