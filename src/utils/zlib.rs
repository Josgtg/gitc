use std::io::{Read, Write};
use std::rc::Rc;

use flate2::Compression;
use flate2::bufread::ZlibDecoder;
use flate2::write::ZlibEncoder;

use crate::Result;
use crate::error::CustomResult;

/// Compresses `bytes` using a zlib encoder.
///
/// # Errors
///
/// This function will fail if the `ZlibEncoder` fails.
pub fn compress(bytes: &[u8]) -> Result<Rc<[u8]>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(bytes)
        .map_err_with("failed to write to encoder when compressing data")?;
    let compressed = encoder
        .finish()
        .map_err_with("could not finalize compression")?
        .into();

    Ok(compressed)
}

/// Returns `bytes` decompressed, using a zlib decoder.
///
/// # Errors
///
/// This function will fail if reading from the bytes was not possible.
pub fn decompress(bytes: &[u8]) -> Result<Rc<[u8]>> {
    let mut buf = Vec::new();
    let mut decoder = ZlibDecoder::new(bytes);
    decoder
        .read_to_end(&mut buf)
        .map_err_with("could not read data when decompressing data")?;

    Ok(buf.into())
}
