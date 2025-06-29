use std::io::Cursor;
use std::io::Write;

use byteorder::WriteBytesExt;

use flate2::{write::ZlibEncoder, Compression};

use crate::byteable::Byteable;
use crate::hashing;
use crate::Result;

#[derive(Debug)]
pub enum ObjectType {
    Blob,
    Tree,
    Commit,
}

impl std::fmt::Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Blob => "blob",
            Self::Tree => "tree",
            Self::Commit => "commit",
        })
    }
}

#[derive(Debug)]
pub struct Object {
    pub kind: ObjectType,
    pub data: Vec<u8>,
}

impl Object {
    pub fn new<T: AsRef<[u8]> + Into<Vec<u8>>>(kind: ObjectType, data: T) -> Self {
        Self {
            kind,
            data: data.into(),
        }
    }

    /// Returns the compressed version of this object, result of encoding it with the `as_bytes`
    /// function and compressing it using zlib.
    ///
    /// # Errors
    ///
    /// This function can fail if it couldn't encode the object or it couldn't write to the
    /// encoder.
    pub fn compress(&self) -> Result<Vec<u8>> {
        let bytes = self.as_bytes()?;
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&bytes)?;
        encoder.finish().map_err(|e| e.into())
    }

    /// Returns the SHA1 hash for this object.
    ///
    /// # Errors
    ///
    /// This function will fail if the object couldn't be compressed.
    pub fn hash(&self) -> Result<Vec<u8>> {
        let compressed = self.compress()?;
        Ok(hashing::hash(&compressed))
    }
}

impl Byteable for Object {

    /// Returns the encoded data for this object, with the following format:
    ///
    /// `{type} {data_length}\0{data}`
    ///
    /// # Errors
    ///
    /// This function will fail if any write operation to a `std::io::Cursor` returns an error.
    fn as_bytes(&self) -> Result<Vec<u8>> {
        // Encoding to this format: blob 4\0abcd
        let mut cursor = Cursor::new(Vec::new());

        cursor.write_all(self.kind.to_string().as_bytes())?;
        cursor.write_u8(' ' as u8)?;
        cursor.write_all(self.data.len().to_string().as_bytes())?;
        cursor.write_u8(0)?;
        cursor.write_all(&self.data)?;

        Ok(cursor.into_inner())
    }

    // TODO
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        todo!()
    }
}
