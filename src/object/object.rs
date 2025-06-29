use std::fs::File;
use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use std::rc::Rc;

use byteorder::WriteBytesExt;

use flate2::{write::ZlibEncoder, Compression};

use crate::byteable::Byteable;
use crate::Result;

use super::ObjectType;

/// Represents an object that would be written to the objects folder.
#[derive(Debug)]
pub struct Object {
    kind: ObjectType,
    data: Rc<[u8]>,
}

impl Object {

    /// Returns the compressed version of this object, result of encoding it with the `as_bytes`
    /// function and compressing it using zlib.
    ///
    /// # Errors
    ///
    /// This function can fail if it couldn't encode the object or it couldn't write to the
    /// encoder.
    pub fn compress(&self) -> Result<Rc<[u8]>> {
        let bytes = self.as_bytes()?;
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&bytes)?;
        encoder.finish().map(|b| b.into()).map_err(|e| e.into())
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
    fn as_bytes(&self) -> Result<Rc<[u8]>> {
        // Encoding to this format: blob 4\0abcd
        let mut cursor = Cursor::new(Vec::new());

        cursor.write_all(self.kind.to_string().as_bytes())?;
        cursor.write_u8(' ' as u8)?;
        cursor.write_all(self.data.len().to_string().as_bytes())?;
        cursor.write_u8(b'\0')?;
        cursor.write_all(&self.data)?;

        Ok(cursor.into_inner().into())
    }

    // TODO
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        todo!()
    }
}

impl TryFrom<File> for Object {
    type Error = crate::Error;

    fn try_from(mut file: File) -> Result<Self> {
        let mut buf = Vec::new();
        file.read(&mut buf)?;
        Ok(Object {
            kind: ObjectType::Blob,
            data: buf.into(),
        })
    }
}
