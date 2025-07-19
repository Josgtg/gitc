use anyhow::{bail, Context, Result};
use std::io::{BufRead, Cursor};
use std::rc::Rc;

use crate::byteable::Byteable;
use crate::hashing::Hash;

use super::commit::CommitUser;
use super::tree::TreeEntry;

use super::blob;
use super::commit;
use super::tree;

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
        parents: Rc<[Hash]>,
        author: CommitUser,
        committer: CommitUser,
        message: Rc<str>,
    },
}

impl Object {
    pub const BLOB_STRING: &str = "blob";
    pub const TREE_STRING: &str = "tree";
    pub const COMMIT_STRING: &str = "commit";

    /// Turns this object into bytes and calls `Hash::new` from said bytes.
    ///
    /// If you have already computed the serialized version of this object, it is better to just
    /// use `Hash::new` directly.
    ///
    /// # Errors
    ///
    /// This function will fail if there was not possible to serialize this object.
    pub fn hash(&self) -> Result<Hash> {
        Ok(Hash::compute(
            &self.as_bytes().context("could not serialize object")?,
        ))
    }
}

impl std::fmt::Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match self {
            Object::Blob { data } => blob::display(data),
            Object::Tree { entries } => tree::display(entries),
            Object::Commit {
                tree,
                parents,
                author,
                committer,
                message,
            } => commit::display(tree, parents, author, committer, message),
        })
    }
}

impl Byteable for Object {
    fn as_bytes(&self) -> Result<Rc<[u8]>> {
        match self {
            Object::Blob { data } => blob::as_bytes(data),
            Object::Tree { entries } => tree::as_bytes(entries),
            Object::Commit {
                tree,
                parents,
                author,
                committer: commiter,
                message,
            } => commit::as_bytes(tree, parents, author, commiter, message),
        }
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Getting object type
        let mut kind_buffer = Vec::new();
        Cursor::new(bytes)
            .read_until(b' ', &mut kind_buffer)
            .context("could not read object header")?;
        kind_buffer.pop().context("expected space after object type")?;

        let kind = String::from_utf8_lossy(&kind_buffer);
        match kind.as_ref() {
            Object::BLOB_STRING => blob::from_bytes(bytes),
            Object::TREE_STRING => tree::from_bytes(bytes),
            Object::COMMIT_STRING => commit::from_bytes(bytes),
            _ => bail!("object did not have a valid type, got: {}", kind),
        }
    }
}
