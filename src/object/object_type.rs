use crate::{Result, Error};

#[derive(Debug)]
pub enum ObjectType {
    Blob,
    Tree,
    Commit,
}

impl ObjectType {

    const BLOB_STRING: &'static str = "blob";
    const TREE_STRING: &'static str = "tree";
    const COMMIT_STRING: &'static str = "commit";
}

impl std::fmt::Display for ObjectType {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Blob => ObjectType::BLOB_STRING,
            Self::Tree => ObjectType::TREE_STRING,
            Self::Commit => ObjectType::COMMIT_STRING,
        })
    }
}

impl TryFrom<&str> for ObjectType {
    type Error = crate::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            ObjectType::BLOB_STRING => Ok(ObjectType::Blob),
            ObjectType::TREE_STRING => Ok(ObjectType::Tree),
            ObjectType::COMMIT_STRING => Ok(ObjectType::Commit),
            _ => Err(Error::Generic("provided value does not match any object type"))
        }
    }
}
