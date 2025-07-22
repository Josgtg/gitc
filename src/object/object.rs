use anyhow::{bail, Context, Result};
use std::io::Cursor;
use std::rc::Rc;

use crate::byteable::Byteable;
use crate::hashing::Hash;
use crate::utils::cursor::EasyRead;

use super::commit::CommitUser;
use super::tree::TreeEntry;

use super::commit;
use super::tree;
use super::{blob, NULL_BYTE, SPACE_BYTE};

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

    /// Tries to read an object header file from a sequence of bytes, returning the type of
    /// object if it had a valid header.
    fn read_header(bytes: &[u8]) -> Result<String> {
        let mut cursor = Cursor::new(bytes);

        let kind_bytes = cursor.read_until_checked(SPACE_BYTE)?;
        let kind = String::from_utf8_lossy(&kind_bytes).to_string();

        // reading lenght just to verify it has a valid header
        let _lenght = cursor.read_until_checked(NULL_BYTE)?;

        Ok(kind)
    }

    /// This function will parse the file, creating a blob object with the whole data, not checking
    /// the header of the file.
    pub fn from_bytes_new_blob(bytes: &[u8]) -> Self {
        Object::Blob { data: bytes.into() }
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

    /// Given a sequence of bytes, tries to read an object header:
    /// - If the header is present, the file would be parsed as the type present in the header.
    /// - If there is no header present, the function will return an error.
    ///
    /// If you want to just read the data as a blob object, use `from_bytes_new_blob`.
    ///
    /// # Errors
    ///
    /// This function would generally fail from parsing errors or from the bytes not having a valid
    /// header.
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let kind = Object::read_header(bytes).context("could not read file header")?;

        match kind.as_ref() {
            Object::BLOB_STRING => blob::from_bytes(bytes),
            Object::TREE_STRING => tree::from_bytes(bytes),
            Object::COMMIT_STRING => commit::from_bytes(bytes),
            _ => bail!("object did not have a valid type, got: {}", kind),
        }
    }
}
