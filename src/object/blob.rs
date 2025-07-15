use std::io::{BufRead, Cursor, Read, Write};
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{Context, Result, anyhow, bail};
use byteorder::WriteBytesExt;

use super::Object;

/// Represents an object with some extra information, like the path.
#[derive(Debug)]
pub struct ExtendedBlob {
    pub blob: Object,
    pub path: PathBuf,
}

/// Returns the encoded version of the bytes of a blob object, following the next format:
///
/// `{type} {data_length}\0{data}`
///
/// # Errors
///
/// This function will fail if any write operation to a `std::io::Cursor` returns an error.
pub fn as_bytes(data: &[u8]) -> Result<Rc<[u8]>> {
    // Encoding to this format: blob 4\0abcd
    let mut cursor = Cursor::new(Vec::new());

    cursor
        .write_all(Object::BLOB_STRING.as_bytes())
        .context("could not write object type")?;
    cursor.write_u8(b' ')?;
    cursor
        .write_all(data.len().to_string().as_bytes())
        .context("could not write object data length")?;
    cursor.write_u8(b'\0')?;
    cursor
        .write_all(data)
        .context("could not write object data")?;

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
pub fn from_bytes(bytes: &[u8]) -> Result<Object> {
    let mut cursor = Cursor::new(bytes);

    // reading kind
    let mut kind_buf = Vec::new();
    cursor
        .read_until(b' ', &mut kind_buf)
        .context("could not read blob's type")?;
    kind_buf.pop(); // popping space character
    let kind = String::from_utf8_lossy(&kind_buf);

    if kind != Object::BLOB_STRING {
        bail!(
            "object type is not {:?}, but {:?}",
            Object::BLOB_STRING,
            kind
        )
    }

    // reading data length
    let mut len_buf = Vec::new();
    cursor
        .read_until(b'\0', &mut len_buf)
        .context("failed to read until null byte when decoding object")?; // reading until null char, before this there is the data length

    if len_buf.pop() != Some(b'\0') {
        bail!("expected null byte after object data length")
    }
    let data_len: usize = String::from_utf8(len_buf)
        .context("failed to build string from object's decoded data length")?
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
