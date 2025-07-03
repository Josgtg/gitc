use std::fs::File;
use std::io::BufRead;
use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use std::rc::Rc;

use byteorder::WriteBytesExt;

use crate::byteable::Byteable;
use crate::error::CustomResult;
use crate::hashing::Hash;
use crate::utils::zlib;
use crate::{Error, Result};

use super::ObjectType;

/// Represents an object that would be written to the objects folder.
#[derive(Debug)]
pub struct Object {
    kind: ObjectType,
    data: Rc<[u8]>,
}

impl Object {
    /// Returns a tuple with the compressed version of this object, result of encoding it with the `as_bytes`
    /// function and compressing it using zlib in the first position and the hash produced by the bytes from
    /// the `as_bytes` function in the second position.
    ///
    /// # Errors
    ///
    /// This function can fail if it couldn't encode the object or it couldn't write to the
    /// encoder.
    pub fn compress(&self) -> Result<Rc<[u8]>> {
        let bytes = self
            .as_bytes()
            .map_err_with("could not encode object when compressing")?;

        zlib::compress(bytes.as_ref()).map_err_with("could not compress object")
    }

    /// Returns an object made from the data, decompressing it and assigning `kind` as its type.
    ///
    /// # Errors
    ///
    /// This function will fail if the decoder couldn't decompress the data.
    pub fn decompress(kind: ObjectType, data: &[u8]) -> Result<Object> {
        Ok(Object {
            kind,
            data: zlib::decompress(data.as_ref())
                .map_err_with("could not decompress data to create an object")?,
        })
    }

    /// Returns the hash for this object binary data.
    pub fn hash(&self) -> Result<Hash> {
        let data = self
            .as_bytes()
            .map_err_with("could not encode object when getting its hash")?;
        Ok(Hash::new(data.as_ref()))
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

        cursor
            .write_all(self.kind.to_string().as_bytes())
            .map_err_with("could not write object type")?;
        cursor.write_u8(b' ')?;
        cursor
            .write_all(self.data.len().to_string().as_bytes())
            .map_err_with("could not write object data length")?;
        cursor.write_u8(b'\0')?;
        cursor
            .write_all(&self.data)
            .map_err_with("could not write object data")?;

        Ok(cursor.into_inner().into())
    }

    // Reads a byte slice, asuming it represents a valid object file.
    //
    // This function also assumes the data is not compressed.
    //
    // # Errors
    //
    // This function will fail if:
    // - The data could not be read.
    // - The data did not have a valid format.
    fn from_bytes<T: BufRead>(cursor: &mut T) -> Result<Self> {
        // reading type
        let mut kind_buf = Vec::new();
        // reading until space, before it there is the object type
        cursor
            .read_until(b' ', &mut kind_buf)
            .map_err_with("failed to read until space when decoding object")?;
        if kind_buf.pop() != Some(b' ') {
            return Err(Error::Formatting("expected space after object type".into()));
        }
        let kind = ObjectType::try_from(
            String::from_utf8(kind_buf)
                .map_err_with("could not get object type from decoded type")?
                .as_str(),
        )?;

        // reading data length
        let mut len_buf = Vec::new();
        cursor
            .read_until(b'\0', &mut len_buf)
            .map_err_with("failed to read until null byte when decoding object")?; // reading until null char, before this there is the data length
        if len_buf.pop() != Some(b'\0') {
            return Err(Error::Formatting(
                "expected null byte after object data length".into(),
            ));
        }
        let data_len: usize = String::from_utf8(len_buf)
            .map_err_with("failed to build string from object's decoded data length")?
            .parse()
            .map_err(|e| {
                Error::DataConsistency(
                    format!("could not read data object lenght as a number: {e:?}").into(),
                )
            })?;

        // reading actual data
        let mut data_buf = Vec::new();
        cursor
            .read_to_end(&mut data_buf)
            .map_err_with("could not read object data when decoding")?;
        if data_len != data_buf.len() {
            return Err(Error::DataConsistency(
                format!(
                    "lenght read \"{}\" did not match actual data length \"{}\"",
                    data_len,
                    data_buf.len()
                )
                .into(),
            ));
        }

        Ok(Object {
            kind,
            data: data_buf.into(),
        })
    }
}

impl TryFrom<File> for Object {
    type Error = crate::Error;

    fn try_from(mut file: File) -> Result<Self> {
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err_with("failed to read from file when building object")?;
        Ok(Object {
            kind: ObjectType::Blob,
            data: buf.into(),
        })
    }
}
