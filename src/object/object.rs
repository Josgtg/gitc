use std::path::PathBuf;
use std::rc::Rc;

use crate::{Error, Result};
use crate::byteable::Byteable;

/// Represents an object with some extra information, like the path.
#[derive(Debug)]
pub struct ExtendedObject {
    pub object: Object,
    pub path: PathBuf,
}

/// Represents the different type of objects there can be: Blobs, Commits and Trees, with methods
/// for byte encoding and decoding.
#[derive(Debug)]
pub enum Object {
    Blob {
        data: Rc<[u8]>,
    },
    Tree {},
    Commit {},
}

impl Object {
    pub const BLOB_STRING: &'static str = "blob";
    pub const TREE_STRING: &'static str = "tree";
    pub const COMMIT_STRING: &'static str = "commit";
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
            Object::Blob { data } => super::blob_as_bytes(data),
            Object::Tree { .. } => Err(Error::NotImplemented),
            Object::Commit { .. } => Err(Error::NotImplemented),
        }
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Must identify object in a way before deciding which function to use
        super::blob_from_bytes(bytes)
    }
}
