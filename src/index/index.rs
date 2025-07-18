use std::io::{Cursor, Read, Write};
use std::rc::Rc;
use std::slice::Iter;

use anyhow::{Context, Result, bail};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::Constants;
use crate::byteable::Byteable;
use crate::hashing::Hash;

use super::{ExtensionEntry, IndexEntry, builder::IndexBuilder};

#[derive(Debug, Clone)]
pub struct Index {
    pub(super) version_number: u32,
    pub(super) entries_number: u32,
    pub(super) entries: Vec<IndexEntry>,
    pub(super) extensions: Vec<ExtensionEntry>,
}

impl Index {
    /// Returns an iterator over the entries of this index.
    pub fn entries(&self) -> Iter<IndexEntry> {
        self.entries.iter()
    }
}

impl Byteable for Index {
    /// Returns a binary representation of this index.
    fn as_bytes(&self) -> Result<Rc<[u8]>> {
        let bytes = Vec::with_capacity(32);

        let mut cursor = Cursor::new(bytes);

        cursor
            .write_u32::<BigEndian>(Constants::INDEX_HEADER_BINARY)
            .context("could not write index header when encoding index")?;
        cursor
            .write_u32::<BigEndian>(self.version_number)
            .context("could not write version_number when encoding index")?;
        cursor
            .write_u32::<BigEndian>(self.entries_number)
            .context("could not write entries_number when encoding index")?;

        let mut entry_data: Rc<[u8]>;
        for e in self.entries.iter() {
            // unwrapping for debugging reasons
            entry_data = e
                .as_bytes()
                .context("could not serialize index entry when encoding index")?;
            cursor
                .write_all(&entry_data)
                .context("could not write index entry encoded data when encoding index")?;
        }

        for _e in self.extensions.iter() {
            // TODO: add code to encode extensions
        }

        // assigning checksum from previous data
        let checksum = Hash::compute(cursor.get_ref());
        cursor
            .write_all(checksum.as_ref())
            .context("could not write checksum when encoding index")?;

        Ok(cursor.into_inner().into())
    }

    /// Parses a set of bytes into an `Index` struct.
    ///
    /// # Errors
    ///
    /// This function will fail if:
    /// - There was an error reading from the bytes.
    /// - The format of the bytes was not the expected one.
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut builder = IndexBuilder::new();
        let mut cursor = Cursor::new(bytes);

        let dirc = cursor
            .read_u32::<BigEndian>()
            .context("could not read DIRC when decoding index")?;
        if dirc != Constants::INDEX_HEADER_BINARY {
            bail!("index file does not contain a valid header")
        }

        let version_number = cursor
            .read_u32::<BigEndian>()
            .context("could not read version_number when decoding index")?;
        if version_number != Constants::INDEX_VERSION_NUMBER {
            bail!(
                "index file version {} is not supported, was expecting version {}",
                version_number,
                Constants::INDEX_VERSION_NUMBER
            )
        }

        let entries_number = cursor
            .read_u32::<BigEndian>()
            .context("could not read entries_number when decoding index")?;

        let mut entry: IndexEntry;
        let mut bytes: &[u8];
        let mut position: usize;
        for _ in 0..entries_number {
            position = cursor.position() as usize;
            bytes = &cursor.get_ref()[position..];
            entry = IndexEntry::from_bytes(bytes)
                .context("failed to build an index entry when decoding index")?;
            // Advancing to the next index entry
            cursor.set_position((position + entry.len()) as u64);
            builder.add_index_entry(entry);
        }

        // TODO: Read extensions

        let index = builder.build();
        let index_bytes = index
            .as_bytes()
            .context("could not encode current index when decoding index")?; // TODO: look for a way to check hash without hashing all the index

        let produced_hash = &index_bytes[index_bytes.len() - 20..];
        let mut actual_hash: [u8; 20] = [0; 20];
        cursor
            .read_exact(&mut actual_hash)
            .context("could not read checksum when decoding index")?;

        // checking for valid checksum
        if produced_hash != actual_hash {
            /*
            return Err(Error::DataConsistency(
                "index checksum does not correspond with internal data".into(),
            ));
            */
            eprintln!("index checksum does not correspond with internal data")
        }

        Ok(index)
    }
}

impl Default for Index {
    fn default() -> Self {
        Index {
            version_number: Constants::INDEX_VERSION_NUMBER,
            entries_number: u32::default(),
            entries: Vec::default(),
            extensions: Vec::default(),
        }
    }
}
