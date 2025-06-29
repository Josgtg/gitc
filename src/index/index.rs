use std::io::Cursor;
use std::io::Write;

use byteorder::BigEndian;
use byteorder::WriteBytesExt;

use crate::byteable::Byteable;
use crate::hashing;
use crate::{Constants, Result};

use super::ExtensionEntry;
use super::IndexEntry;

pub struct Index {
    version_number: u32,
    entries_number: u32,
    entries: Vec<IndexEntry>,
    extensions: Vec<ExtensionEntry>,
}

impl Byteable for Index {

    /// Returns a binary representation of this index.
    fn as_bytes(&self) -> Result<Vec<u8>> {
        let bytes = Vec::with_capacity(32);

        let mut cursor = Cursor::new(bytes);

        cursor.write_u32::<BigEndian>(Constants::INDEX_HEADER_BINARY)?;
        cursor.write_u32::<BigEndian>(self.version_number)?;
        cursor.write_u32::<BigEndian>(self.entries_number)?;

        let mut current_len: usize = 12;  // 12 from the 3 bytes above
        let mut entry_data: Vec<u8>;
        let mut offset: usize;
        for e in self.entries.iter() {
            // unwrapping for debugging reasons
            entry_data = e.as_bytes().unwrap();
            cursor.write_all(&entry_data)?;

            current_len += entry_data.len();
            offset = 8 - ((8 % current_len) % 8);
            for _ in 0..offset {
                cursor.write_u8(0)?;
            }
        }

        for _e in self.extensions.iter() {
            // TODO: add code to encode extensions
        }

        // assigning checksum from previous data
        let checksum = hashing::hash(cursor.get_ref());
        cursor.write_all(&checksum)?;

        Ok(cursor.into_inner())
    }

    /// Parses a set of bytes into an `Index` struct.
    ///
    /// # Errors
    ///
    /// This function will fail if:
    /// - There was an error reading from the bytes.
    /// - The format of the bytes was not the expected one.
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        todo!()
    }
}
