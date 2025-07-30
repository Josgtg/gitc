use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{bail, Context, Result, anyhow};

use crate::utils::cursor::EasyRead;

use super::*;

/// Represents an object with some extra information, like the path.
#[derive(Debug)]
pub struct BlobExt {
    pub blob: Object,
    pub path: PathBuf,
}

impl BlobExt {
    /// Reads the data from a file in `path` and returns a blob object containing said data.
    ///
    /// # Errors
    ///
    /// This function will fail if the `path` could not be read from.
    pub fn from_file(path: PathBuf) -> Result<Self> {
        let data = std::fs::read(&path).context("could not read file")?;
        Ok(BlobExt {
            blob: Object::from_bytes_new_blob(&data),
            path,
        })
    }

    /// Returns the data stored inside the inner blob object
    pub fn data(self) -> Rc<[u8]> {
        if let Object::Blob { data } = self.blob {
            data
        } else {
            panic!("BlobExt did not store a blob")
        }
    }
}

/// Returns the encoded version of the bytes of a blob object, following the next format:
///
/// `{type} {data_length}\0{data}`
///
/// # Errors
///
/// This function will fail if any write operation to a `std::io::Cursor` returns an error.
pub fn as_bytes(data: &[u8]) -> Result<Rc<[u8]>> {
    let mut header = format!("{} {}\0", Object::BLOB_STRING, data.len())
        .as_bytes()
        .to_vec();
    header.extend(data);
    Ok(header.into())
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
pub fn from_bytes(bytes: &[u8]) -> Result<Object> {
    let mut cursor = Cursor::new(bytes);

    // reading kind
    let kind = String::from_utf8_lossy(&cursor.read_until_checked(SPACE_BYTE)?).to_string();
    if kind != Object::BLOB_STRING {
        bail!(
            "object type is not {:?}, but {:?}",
            Object::BLOB_STRING,
            kind
        )
    }

    // reading data length
    let len_str = String::from_utf8_lossy(&cursor.read_until_checked(NULL_BYTE)?).to_string();
    let data_len: usize = len_str
        .parse()
        .map_err(|e| anyhow!("could not read data object lenght as a number: {:?}", e))?;

    // reading actual data
    let mut data_buf = Vec::with_capacity(data_len);
    cursor
        .read_to_end(&mut data_buf)
        .context("could not read object data when decoding")?;

    if data_len != data_buf.len() {
        bail!(
            "lenght read \"{}\" did not match actual data length \"{}\"",
            data_len,
            data_buf.len()
        )
    }

    Ok(Object::Blob {
        data: data_buf.into(),
    })
}

pub fn display(data: &[u8]) -> String {
    String::from_utf8_lossy(data).to_string()
}
