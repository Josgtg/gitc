use std::fs::File;
use std::io::{BufRead, Cursor, Read, Write};
use std::rc::Rc;

use anyhow::{Context, Result, anyhow, bail};
use byteorder::WriteBytesExt;

use crate::byteable::Byteable;
use crate::utils::zlib;

use super::Object;

/// Returns the compressed version of this object, result of encoding it with the `as_bytes`
/// function and compressing it using zlib.
///
/// # Errors
///
/// This function can fail if it couldn't encode the object or it couldn't write to the
/// encoder.
pub fn compress_blob(blob: Object) -> Result<Rc<[u8]>> {
    // Returning if this object is not a blob
    match blob {
        Object::Blob { .. } => (), // Ok
        _ => bail!("object is not a blob"),
    }

    let bytes = blob
        .as_bytes()
        .context("could not encode object when compressing")?;

    zlib::compress(bytes.as_ref()).context("could not compress object")
}

/// Returns a blob object made from the data, decompressing it.
///
/// # Errors
///
/// This function will fail if the decoder couldn't decompress the data.
pub fn decompress_blob(data: &[u8]) -> Result<Object> {
    Ok(Object::Blob {
        data: zlib::decompress(data.as_ref())
            .context("could not decompress data to create an object")?,
    })
}

/// Returns the encoded data for this object, with the following format:
///
/// `{type} {data_length}\0{data}`
///
/// # Errors
///
/// This function will fail if any write operation to a `std::io::Cursor` returns an error.
pub fn blob_as_bytes(data: &[u8]) -> Result<Rc<[u8]>> {
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
pub fn blob_from_bytes(bytes: &[u8]) -> Result<Object> {
    let mut cursor = Cursor::new(bytes);

    // reading kind
    let mut kind_buf = Vec::new();
    cursor
        .read_until(b' ', &mut kind_buf)
        .context("could not read blob's type")?;
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
    let mut data_buf = Vec::new();
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

pub fn blob_try_from_file(mut file: File) -> Result<Object> {
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .context("failed to read from file when building object")?;
    Ok(Object::Blob { data: buf.into() })
}
