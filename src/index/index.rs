use std::io::{BufRead, Cursor, Write};
use std::rc::Rc;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::byteable::Byteable;
use crate::{Constants, Result, Error};
use crate::hashing::Hash;

use super::{ExtensionEntry, IndexBuilder, IndexEntry};

#[derive(Debug, Default, Clone)]
pub struct Index {
    pub(super) version_number: u32,
    pub(super) entries_number: u32,
    pub(super) entries: Vec<IndexEntry>,
    pub(super) extensions: Vec<ExtensionEntry>,
}

impl Byteable for Index {
    /// Returns a binary representation of this index.
    fn as_bytes(&self) -> Result<Rc<[u8]>> {
        let bytes = Vec::with_capacity(32);

        let mut cursor = Cursor::new(bytes);

        cursor.write_u32::<BigEndian>(Constants::INDEX_HEADER_BINARY)?;
        cursor.write_u32::<BigEndian>(self.version_number)?;
        cursor.write_u32::<BigEndian>(self.entries_number)?;

        let mut current_len: usize = 12; // 12 from the 3 bytes above
        let mut entry_data: Rc<[u8]>;
        let mut offset: usize;
        for e in self.entries.iter() {
            // unwrapping for debugging reasons
            entry_data = e.as_bytes().expect("could not serialize index entry");
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
        let checksum = Hash::from(cursor.get_ref());
        cursor.write_all(checksum.as_ref())?;

        Ok(cursor.into_inner().into())
    }

    /// Parses a set of bytes into an `Index` struct.
    ///
    /// # Errors
    ///
    /// This function will fail if:
    /// - There was an error reading from the bytes.
    /// - The format of the bytes was not the expected one.
    fn from_bytes<R: BufRead>(cursor: &mut R) -> Result<Self> {
        let mut builder = IndexBuilder::new();

        let dirc = cursor.read_u32::<BigEndian>()?;
        if dirc != Constants::INDEX_HEADER_BINARY {
            return Err(Error::DataConsistency(
                "index file does not contain a valid header".into(),
            ));
        }

        let version_number = cursor.read_u32::<BigEndian>()?;
        if version_number != Constants::INDEX_VERSION_NUMBER {
            return Err(Error::DataConsistency(
                format!(
                    "index file version {} is not supported, was expecting version {}",
                    version_number,
                    Constants::INDEX_VERSION_NUMBER
                )
                .into(),
            ));
        }

        let entries_number = cursor.read_u32::<BigEndian>()?;

        let mut entry: IndexEntry;
        let mut current_len: usize = 12;
        let mut padding: usize;
        for _ in 0..entries_number {
            entry = IndexEntry::from_bytes(cursor)
                .expect("could not form an index entry from the given cursor");

            current_len += entry.len();
            padding = (8 - (current_len % 8)) % 8;
            for _ in 0..padding {
                cursor.read_u8()?;
            }

            builder.add_index_entry(entry);
        }

        // TODO: Read extensions

        let index = builder.build();
        let index_bytes = index.as_bytes().expect("could not serialize index"); // TODO: look for a way to check hash without hashing all the index

        let produced_hash = &index_bytes[index_bytes.len() - 20..];
        let mut actual_hash: [u8; 20] = [0; 20];
        cursor.read_exact(&mut actual_hash)?;

        if produced_hash != actual_hash {
            return Err(Error::DataConsistency(
                "index checksum does not correspond with internal data".into(),
            ));
        }

        Ok(index)
    }
}
